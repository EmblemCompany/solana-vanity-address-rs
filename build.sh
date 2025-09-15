#!/bin/bash
# Custom build script for Heroku
echo "=== Custom Build Script ==="
echo "Building with API features..."
cargo build --release --features api --bin solana-vanity-api
echo "Build complete!"
echo "Checking for binary:"
ls -la target/release/solana-vanity*
# Copy to expected location
if [ -f "target/release/solana-vanity-api" ]; then
    echo "Copying solana-vanity-api to root directory"
    cp target/release/solana-vanity-api ./solana-vanity-api
    chmod +x ./solana-vanity-api
fi