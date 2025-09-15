#!/bin/bash
# Find and run the solana-vanity-api binary
if [ -f "./target/release/solana-vanity-api" ]; then
    exec ./target/release/solana-vanity-api
elif [ -f "./solana-vanity-api" ]; then
    exec ./solana-vanity-api
else
    echo "Error: solana-vanity-api binary not found!"
    ls -la
    ls -la target/release/ 2>/dev/null || echo "No target/release directory"
    exit 1
fi