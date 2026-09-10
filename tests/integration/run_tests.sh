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

TEST_EMAIL="integration-${GITHUB_RUN_ID:-local}-$(date +%s)@example.com"
TEST_PASSWORD='integration-test-password'

echo '=== Test 1 : Health check ==='
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' \
    --cacert $CA_CERT --cert $CLIENT_CERT --key $CLIENT_KEY \
    $BASE_URL/health)

assert_eq 'GET /health → 200' '200' "$STATUS"

echo '=== Authentification de test ==='
curl -sf -X POST "$BASE_URL/auth/register" \
    --cacert "$CA_CERT" \
    --cert "$CLIENT_CERT" --key "$CLIENT_KEY" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$TEST_EMAIL\",\"fullname\":\"Integration Test\",\"username\":\"integration-${GITHUB_RUN_ID:-local}\",\"password\":\"$TEST_PASSWORD\"}" \
    > /dev/null
LOGIN_RESPONSE=$(curl -sf -X POST "$BASE_URL/auth/login" \
    --cacert "$CA_CERT" \
    --cert "$CLIENT_CERT" --key "$CLIENT_KEY" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}")
TOKEN=$(echo "$LOGIN_RESPONSE" | jq -er '.token')

echo '=== Test 2 : Upload d un fichier ==='
dd if=/dev/urandom of=/tmp/test-upload.bin bs=1M count=5 2>/dev/null
EXPECTED_SHA=$(sha256sum /tmp/test-upload.bin | cut -d' ' -f1)
UPLOAD_STATUS=$(curl -sS -o /tmp/upload-response.json -w '%{http_code}' -X POST \
    --cacert $CA_CERT \
    --cert $CLIENT_CERT --key $CLIENT_KEY \
    -H "Authorization: Bearer $TOKEN" \
    -F 'file=@/tmp/test-upload.bin' \
    $BASE_URL/files)
if [ "$UPLOAD_STATUS" != '201' ]; then
    echo "Upload failed with HTTP $UPLOAD_STATUS:"
    cat /tmp/upload-response.json
    exit 1
fi
RESPONSE=$(cat /tmp/upload-response.json)
RETURNED_SHA=$(echo $RESPONSE | jq -r '.sha256')
assert_eq 'SHA-256 intégrité upload' "$EXPECTED_SHA" "$RETURNED_SHA"
FILE_ID=$(echo $RESPONSE | jq -r '.id')

echo '=== Test 3 : Liste des fichiers ==='
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' \
    --cacert $CA_CERT --cert $CLIENT_CERT --key $CLIENT_KEY \
    -H "Authorization: Bearer $TOKEN" \
    $BASE_URL/files)
assert_eq 'GET /fichiers → 200' '200' "$STATUS"

echo '=== Test 4 : Download et vérification SHA-256 ==='
curl -sf --cacert $CA_CERT --cert $CLIENT_CERT --key $CLIENT_KEY \
    -o /tmp/test-download.bin \
    -H "Authorization: Bearer $TOKEN" \
    $BASE_URL/files/$FILE_ID
DOWNLOADED_SHA=$(sha256sum /tmp/test-download.bin | cut -d' ' -f1)
assert_eq 'SHA-256 intégrité download' "$EXPECTED_SHA" "$DOWNLOADED_SHA"

echo '=== Test 5 : Authentification mTLS ==='
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' \
    --cacert $CA_CERT \
    $BASE_URL/files || true)
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