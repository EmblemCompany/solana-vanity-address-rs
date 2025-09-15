#!/bin/bash
echo "=== Custom Heroku Build ==="
cargo build --release --features api --bin solana-vanity-api
cp target/release/solana-vanity-api ./solana-vanity-api 2>/dev/null || true
ls -la target/release/