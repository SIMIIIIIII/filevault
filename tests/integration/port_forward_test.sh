#!/usr/bin/env bash
set -euo pipefail

source "$(dirname -- "${BASH_SOURCE[0]}")/port_forward.sh"

assert_port() {
    local expected="$1" line="$2" actual
    actual=$(forwarded_port <(printf '%s\n' "$line"))
    if [ "$actual" != "$expected" ]; then
        printf 'Expected port %s, got %s from %s\n' "$expected" "$actual" "$line" >&2
        exit 1
    fi
}

assert_port 35649 'Forwarding from 127.0.0.1:35649 -> 8081'
assert_port 49152 'Forwarding from 127.0.0.1:49152 -> 8443'
assert_port '' 'Forwarding from 127.0.0.1:bad -> 8081'
assert_port '' 'Forwarding from [::1]:35649 -> 8081'
echo 'Port-forward output parsing passed.'
