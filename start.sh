#!/bin/bash
echo "=== Starting Solana Vanity API ==="
echo "Current directory: $(pwd)"
echo "Directory contents:"
ls -la

echo ""
echo "Looking for binary in target/release:"
if [ -d "target/release" ]; then
    ls -la target/release/
    if [ -f "target/release/solana-vanity-api" ]; then
        echo "Found binary at target/release/solana-vanity-api"
        exec ./target/release/solana-vanity-api
    fi
fi

echo ""
echo "Looking for binary in root:"
if [ -f "solana-vanity-api" ]; then
    echo "Found binary at ./solana-vanity-api"
    exec ./solana-vanity-api
fi

echo ""
echo "Looking for any solana-vanity* executables:"
find . -name "solana-vanity*" -type f -executable 2>/dev/null

echo ""
echo "ERROR: Could not find solana-vanity-api binary!"
echo "Build may have failed or binary has different name"
exit 1