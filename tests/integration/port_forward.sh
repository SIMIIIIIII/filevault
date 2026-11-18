#!/usr/bin/env bash

forwarded_port() {
    awk '/^Forwarding from 127\.0\.0\.1:[0-9]+ -> [0-9]+$/ {
        split($3, address, ":")
        print address[2]
        exit
    }' "$1"
}
