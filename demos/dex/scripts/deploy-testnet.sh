#!/bin/bash
# Deploy Zeus DEX to Ethereum testnet

echo "Deploying Zeus DEX to Sepolia testnet..."

# Environment check
if [ -z "$PRIVATE_KEY" ]; then
    echo "Error: Set PRIVATE_KEY environment variable"
    exit 1
fi

# Deploy
cd /Users/shy/Developer/ZEUS/demos/dex
npx hardhat run scripts/deploy.js --network sepolia

echo "✅ DEX deployed"
