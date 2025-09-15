#!/bin/bash

# Script to generate multiple Solana addresses ending with "bonk"
# Usage: ./generate_bonk_addresses.sh [number_of_addresses] [threads]

# Default values
COUNT=${1:-10}  # Default to 10 addresses if not specified
THREADS=${2:-64}  # Default to 64 threads if not specified
OUTPUT_FILE="bonk_addresses.txt"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Generating $COUNT Solana addresses ending with 'bonk' using $THREADS threads...${NC}"
echo "Results will be saved to: $OUTPUT_FILE"
echo "========================================" | tee -a "$OUTPUT_FILE"
echo "Generated on: $(date)" | tee -a "$OUTPUT_FILE"
echo "========================================" | tee -a "$OUTPUT_FILE"

# Source cargo environment
source "$HOME/.cargo/env"

# Counter for successful generations
generated=0

while [ $generated -lt $COUNT ]; do
    echo -e "\n${BLUE}Searching for address $((generated + 1)) of $COUNT...${NC}"

    # Run the vanity generator and capture output
    output=$(./target/release/solana-vanity --suffix bonk --threads "$THREADS" 2>&1)

    if echo "$output" | grep -q "Found a vanity address"; then
        # Extract the address and private key
        address=$(echo "$output" | grep "Address:" | awk '{print $2}')
        private_key=$(echo "$output" | grep "Private Key (Base58):" | awk '{print $4}')

        # Increment counter
        ((generated++))

        # Display to console
        echo -e "${GREEN}✓ Found #$generated: $address${NC}"

        # Save to file
        echo "" >> "$OUTPUT_FILE"
        echo "Entry #$generated" >> "$OUTPUT_FILE"
        echo "Address: $address" >> "$OUTPUT_FILE"
        echo "Private Key: $private_key" >> "$OUTPUT_FILE"
        echo "----------------------------------------" >> "$OUTPUT_FILE"
    else
        echo "Error occurred, retrying..."
    fi
done

echo -e "\n${GREEN}Successfully generated $COUNT addresses ending with 'bonk'!${NC}"
echo "All addresses saved to: $OUTPUT_FILE"