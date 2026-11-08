#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
workdir="$(mktemp -d)"
namespace="filevault-ci-${GITHUB_RUN_ID:-local}-$$"
port_forward_pid=""

cleanup() {
    status=$?
    if [ "$status" -ne 0 ]; then
        kubectl -n "$namespace" get pods || true
        kubectl -n "$namespace" get events --sort-by=.metadata.creationTimestamp || true
        kubectl -n "$namespace" logs deploy/postgres --all-containers=true || true
        kubectl -n "$namespace" logs deploy/filevault-filevault --all-containers=true || true
        test ! -f "$workdir/port-forward.log" || cat "$workdir/port-forward.log"
    fi
    if [ -n "$port_forward_pid" ]; then
        kill "$port_forward_pid" 2>/dev/null || true
        wait "$port_forward_pid" 2>/dev/null || true
    fi
    kubectl delete namespace "$namespace" --wait=false --ignore-not-found || true
    rm -f "$workdir/generate_pki.sh" "$workdir/port-forward.log" \
        "$workdir/pki/ca/ca.key" "$workdir/pki/ca/ca.crt" "$workdir/pki/ca/ca.srl" \
        "$workdir/pki/server/server.key" "$workdir/pki/server/server.crt" "$workdir/pki/server/server.csr" \
        "$workdir/pki/client/client.key" "$workdir/pki/client/client.crt" "$workdir/pki/client/client.csr" \
        "$workdir/pki/service/service.key" "$workdir/pki/service/service.crt" "$workdir/pki/service/service.csr"
    rmdir "$workdir/pki/ca" "$workdir/pki/server" "$workdir/pki/client" "$workdir/pki/service" "$workdir/pki" "$workdir" 2>/dev/null || true
    exit "$status"
}
trap cleanup EXIT

cp "$root/generate_pki.sh" "$workdir/"
cd "$workdir"
bash ./generate_pki.sh

kubectl create namespace "$namespace"
kubectl -n "$namespace" create secret generic db-url \
    --from-literal=DATABASE_URL='postgresql://filevault:testpassword@postgres:5432/filevault'
kubectl -n "$namespace" create secret generic jwt-secret --from-literal=JWT_SECRET=integration-test-only
kubectl -n "$namespace" create secret generic filevault-pki \
    --from-file=ca.crt=pki/ca/ca.crt \
    --from-file=server.crt=pki/server/server.crt \
    --from-file=server.key=pki/server/server.key
kubectl -n "$namespace" create configmap postgres-migrations --from-file="$root/migrations"

kubectl -n "$namespace" apply -f - <<'YAML'
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: filevault-files
spec:
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: 1Gi
---
apiVersion: v1
kind: Service
metadata:
  name: postgres
spec:
  selector:
    app: postgres
  ports:
    - port: 5432
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: postgres
spec:
  replicas: 1
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
        - name: postgres
          image: postgres:16
          env:
            - name: POSTGRES_USER
              value: filevault
            - name: POSTGRES_PASSWORD
              value: testpassword
            - name: POSTGRES_DB
              value: filevault
          readinessProbe:
            exec:
              command: [pg_isready, -U, filevault, -d, filevault]
            periodSeconds: 2
          volumeMounts:
            - name: migrations
              mountPath: /docker-entrypoint-initdb.d
              readOnly: true
      volumes:
        - name: migrations
          configMap:
            name: postgres-migrations
YAML

kubectl -n "$namespace" rollout status deployment/postgres --timeout=180s
helm lint "$root/charts/filevault" -f "$root/charts/filevault/values-prod.yaml"
helm upgrade --install filevault "$root/charts/filevault" \
    -n "$namespace" -f "$root/charts/filevault/values-prod.yaml" \
    --set-string image.repository=filevault-kind-ci \
    --set-string image.tag=local --set image.pullPolicy=Never \
    --wait --timeout 180s
kubectl -n "$namespace" rollout status deployment/filevault-filevault --timeout=180s
kubectl -n "$namespace" port-forward --address 127.0.0.1 svc/filevault-filevault :8443 > "$workdir/port-forward.log" 2>&1 &
port_forward_pid=$!
for i in $(seq 1 30); do
    port=$(sed -n 's/^Forwarding from 127\.0\.0\.1:\([0-9]*\) -> 8443$/\1/p' "$workdir/port-forward.log" | head -n 1)
    if [ -n "$port" ] && curl -sf --cacert pki/ca/ca.crt --cert pki/client/client.crt --key pki/client/client.key \
        "https://127.0.0.1:$port/health" > /dev/null; then
        export BASE_URL="https://127.0.0.1:$port"
        bash "$root/tests/integration/run_tests.sh"
        exit 0
    fi
    sleep 2
done
echo "API was not reachable through the Kubernetes Service" >&2
exit 1
