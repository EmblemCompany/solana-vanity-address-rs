#!/bin/bash
echo "=== Starting Solana Vanity API ==="
echo "Current directory: $(pwd)"
echo "Directory contents:"
ls -la

echo ""
echo "Looking for binary in target/release:"
if [ -d "target/release" ]; then
    ls -la target/release/
    # Check for solana-vanity-api first (preferred)
    if [ -f "target/release/solana-vanity-api" ]; then
        echo "Found API binary at target/release/solana-vanity-api"
        exec ./target/release/solana-vanity-api
    # Fall back to solana-vanity if API binary not found
    elif [ -f "target/release/solana-vanity" ]; then
        echo "WARNING: Found CLI binary but not API binary!"
        echo "The build didn't create solana-vanity-api"
        echo "Buildpack may not be using --features api flag"
        exit 1
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