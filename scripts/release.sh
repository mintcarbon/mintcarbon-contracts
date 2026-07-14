#!/bin/bash
set -euo pipefail

NETWORK="testnet"
DRY_RUN=false
FORCE=false

usage() {
    echo "Usage: $0 [--network <testnet|mainnet>] [--dry-run] [--force]"
    echo ""
    echo "Options:"
    echo "  --network   Target network (default: testnet)"
    echo "  --dry-run   Build and validate without deploying"
    echo "  --force     Skip confirmation prompt"
    exit 1
}

while [[ "$#" -gt 0 ]]; do
    case $1 in
        --network) NETWORK="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --force) FORCE=true; shift ;;
        -h|--help) usage ;;
        *) echo "Unknown parameter: $1"; usage ;;
    esac
done

if [[ "$NETWORK" != "testnet" && "$NETWORK" != "mainnet" ]]; then
    echo "Error: network must be 'testnet' or 'mainnet'"
    exit 1
fi

if [[ "$NETWORK" == "mainnet" && "$DRY_RUN" == false ]]; then
    echo "WARNING: You are about to deploy to MAINNET."
    if [[ "$FORCE" == false ]]; then
        read -p "Type 'yes' to confirm: " confirm
        if [[ "$confirm" != "yes" ]]; then
            echo "Aborted."
            exit 1
        fi
    fi
fi

echo "==> Building contracts for $NETWORK..."

cargo build --target wasm32-unknown-unknown --release

CONTRACTS=("audit_log" "verification_records" "carbon_token" "escrow" "marketplace" "governance")

echo "==> Validating WASM artifacts..."
for name in "${CONTRACTS[@]}"; do
    wasm_path="target/wasm32-unknown-unknown/release/${name}.wasm"
    if [[ ! -f "$wasm_path" ]]; then
        echo "Error: $wasm_path not found"
        exit 1
    fi
    size=$(wc -c < "$wasm_path")
    echo "  $name.wasm  (${size} bytes)"
done

if [[ "$DRY_RUN" == true ]]; then
    echo "==> Dry run complete. No contracts deployed."
    exit 0
fi

echo "==> Deploying to $NETWORK..."

if ! command -v soroban &> /dev/null; then
    echo "Error: soroban CLI not found. Install it from https://soroban.stellar.org"
    exit 1
fi

for name in "${CONTRACTS[@]}"; do
    wasm_path="target/wasm32-unknown-unknown/release/${name}.wasm"
    echo "  Deploying $name..."
    contract_id=$(soroban contract deploy \
        --wasm "$wasm_path" \
        --network "$NETWORK" \
        --source-account "$SOURCE_ACCOUNT" 2>&1) || {
        echo "Error deploying $name: $contract_id"
        exit 1
    }
    echo "  $name deployed at: $contract_id"
done

echo "==> All contracts deployed to $NETWORK successfully."
