#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PKI_DIR="${SCRIPT_DIR}/pki"

umask 077

mkdir -p "$PKI_DIR/ca" "$PKI_DIR/server" "$PKI_DIR/client" "$PKI_DIR/service"

rm -f \
  "$PKI_DIR/ca/ca.key" "$PKI_DIR/ca/ca.crt" "$PKI_DIR/ca/ca.srl" \
  "$PKI_DIR/server/server.key" "$PKI_DIR/server/server.crt" "$PKI_DIR/server/server.csr" "$PKI_DIR/server/server.srl" \
  "$PKI_DIR/client/client.key" "$PKI_DIR/client/client.crt" "$PKI_DIR/client/client.csr" "$PKI_DIR/client/client.srl" \
  "$PKI_DIR/service/service.key" "$PKI_DIR/service/service.crt" "$PKI_DIR/service/service.csr" "$PKI_DIR/service/service.srl"

create_leaf_cert() {
  local name="$1"
  local cn="$2"
  local eku="$3"
  local san="$4"
  local cert_dir="$PKI_DIR/$name"
  local ext_file
  ext_file="$(mktemp)"

  openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:prime256v1 -out "$cert_dir/${name}.key"

  openssl req -new \
    -key "$cert_dir/${name}.key" \
    -out "$cert_dir/${name}.csr" \
    -subj "/CN=${cn}/O=UCLouvain/C=BE"

  cat > "$ext_file" <<EOF
basicConstraints=CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=${eku}
subjectAltName=${san}
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid,issuer
EOF

  openssl x509 -req -sha256 -days 365 \
    -in "$cert_dir/${name}.csr" \
    -CA "$PKI_DIR/ca/ca.crt" \
    -CAkey "$PKI_DIR/ca/ca.key" \
    -CAcreateserial \
    -out "$cert_dir/${name}.crt" \
    -extfile "$ext_file"

  rm -f "$ext_file"
}

######### 1. CA RACINE ########
echo '=== Generate root CA ==='
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:secp384r1 -out "$PKI_DIR/ca/ca.key"
chmod 600 "$PKI_DIR/ca/ca.key"

openssl req -new -x509 -sha384 -days 3650 \
  -key "$PKI_DIR/ca/ca.key" \
  -out "$PKI_DIR/ca/ca.crt" \
  -subj '/CN=FileVault-CA/O=UCLouvain/C=BE' \
  -addext 'basicConstraints=critical,CA:TRUE' \
  -addext 'keyUsage=critical,keyCertSign,cRLSign' \
  -addext 'subjectKeyIdentifier=hash'

######### 2. SERVER CERTIFICAT ########
echo '=== Certificat server ==='
create_leaf_cert "server" "filevault.local" "serverAuth" "DNS:filevault.local,DNS:localhost,IP:127.0.0.1"

######### 3. CLIENT CERTIFICAT ########
echo '=== Certificat client CLI ==='
create_leaf_cert "client" "filevault-cli" "clientAuth" "DNS:localhost,IP:127.0.0.1"

######### 4. SERVICE CERTIFICAT (facultatif) ########
echo '=== Certificat service ==='
create_leaf_cert "service" "filevault-service" "serverAuth" "DNS:filevault-service,DNS:localhost,IP:127.0.0.1"

echo '=== PKI générée dans ./pki/ ==='
echo "CA: ${PKI_DIR}/ca/ca.crt + ${PKI_DIR}/ca/ca.key"
echo "Serveur: ${PKI_DIR}/server/server.crt + ${PKI_DIR}/server/server.key"
echo "Client: ${PKI_DIR}/client/client.crt + ${PKI_DIR}/client/client.key"
echo "Service: ${PKI_DIR}/service/service.crt + ${PKI_DIR}/service/service.key"

