#!/bin/bash

set -euo pipefail
BASE_URL='https://127.0.0.1:8443'
CA_CERT='pki/ca/ca.crt'
CLIENT_CERT='pki/client/client.crt'
CLIENT_KEY='pki/client/client.key'
PASS=0; FAIL=0

assert_eq() {
    local desc=$1 expected=$2 actual=$3
    if [ "$actual" = "$expected" ]; then
        echo " PASS : $desc"
        PASS=$((PASS+1))
    else
        echo " FAIL : $desc"
        echo " attendu: $expected, obtenu: $actual"
        FAIL=$((FAIL+1))
    fi
}

echo '=== Test 1 : Health check ==='
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' \
    --cacert $CA_CERT $BASE_URL/health)

assert_eq 'GET /health → 200' '200' "$STATUS"

echo '=== Test 2 : Upload d un fichier ==='
dd if=/dev/urandom of=/tmp/test-upload.bin bs=1M count=5 2>/dev/null
EXPECTED_SHA=$(sha256sum /tmp/test-upload.bin | cut -d' ' -f1)
RESPONSE=$(curl -sf -X POST \
    --cacert $CA_CERT \
    --cert $CLIENT_CERT --key $CLIENT_KEY \
    -F 'file=@/tmp/test-upload.bin' \
    $BASE_URL/files)
RETURNED_SHA=$(echo $RESPONSE | jq -r '.sha256')
assert_eq 'SHA-256 intégrité upload' "$EXPECTED_SHA" "$RETURNED_SHA"
FILE_ID=$(echo $RESPONSE | jq -r '.id')

echo '=== Test 3 : Liste des fichiers ==='
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' \
    --cacert $CA_CERT --cert $CLIENT_CERT --key $CLIENT_KEY \
    $BASE_URL/files)
assert_eq 'GET /fichiers → 200' '200' "$STATUS"

echo '=== Test 4 : Download et vérification SHA-256 ==='
curl -sf --cacert $CA_CERT --cert $CLIENT_CERT --key $CLIENT_KEY \
    -o /tmp/test-download.bin \
    $BASE_URL/files/$FILE_ID
DOWNLOADED_SHA=$(sha256sum /tmp/test-download.bin | cut -d' ' -f1)
assert_eq 'SHA-256 intégrité download' "$EXPECTED_SHA" "$DOWNLOADED_SHA"

echo '=== Test 5 : Authentification mTLS ==='
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' \
    --cacert $CA_CERT \
    $BASE_URL/files 2>&1 || echo '000')
assert_eq 'Sans cert client → rejet' '000' "$STATUS"

echo '=== Test 6 : Rate limiting ==='
for i in $(seq 1 110); do
    curl -sf -o /dev/null --cacert $CA_CERT \
        --cert $CLIENT_CERT --key $CLIENT_KEY \
        $BASE_URL/health
done

STATUS=$(curl -sf -o /dev/null -w '%{http_code}' \
    --cacert $CA_CERT --cert $CLIENT_CERT --key $CLIENT_KEY \
    $BASE_URL/health)
assert_eq 'Rate limit → 429' '429' "$STATUS"

echo ''
echo "=== Résultats : $PASS réussis, $FAIL échoués ==="
[ $FAIL -eq 0 ] || exit 1