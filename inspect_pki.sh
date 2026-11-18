#!/usr/bin/env bash

# Verify CA
openssl verify -CAfile pki/ca/ca.crt pki/server/server.crt


# Verify certificat client
openssl verify -CAfile pki/ca/ca.crt pki/client/client.crt

# See valitity dates
openssl x509 -in pki/server/server.crt -noout -dates

# Verify SAN
openssl x509 -in pki/server/server.crt -noout -ext subjectAltName

# Verify certificat
openssl x509 -in pki/server/server.crt -pubkey -noout | md5sum
openssl ec -in pki/server/server.key -pubout | md5sum

# Test connexion TLS
openssl s_client -connect filevault.local:8443 \
-CAfile pki/ca/ca.crt -servername filevault.local