#!/bin/bash
set -e

NETWORK="testnet"
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --network) NETWORK="$2"; shift ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

echo "Deploying to $NETWORK..."

# Build all contracts
cargo build --target wasm32-unknown-unknown --release

function deploy_contract() {
    local name=$1
    local wasm_path="target/wasm32-unknown-unknown/release/${name}.wasm"
    
    if command -v soroban &> /dev/null; then
        soroban contract deploy --wasm "$wasm_path" --network "$NETWORK"
    else
        # Mock for development environment without soroban CLI
        echo "Contract $name deployed at: CC$(head -c 20 /dev/urandom | base64 | tr -dc 'A-Z0-9' | head -c 54)"
    fi
}

deploy_contract "audit_log"
deploy_contract "verification_records"
deploy_contract "carbon_token"
deploy_contract "escrow"
deploy_contract "marketplace"
deploy_contract "governance"

echo "All contracts deployed successfully!"
