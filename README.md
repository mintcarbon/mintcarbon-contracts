# mintcarbon — contracts

> Soroban smart contracts powering the mintcarbon platform — tokenization, marketplace, escrow, governance, and on-chain verification records for verified carbon credits.

## Overview

This repository contains all on-chain Soroban smart contracts for the mintcarbon platform. The contracts are written in Rust using the Soroban SDK and deployed on the Stellar network.

## Contracts

### CarbonCreditToken

A multi-token contract (analogous to ERC-1155) that manages carbon credit tokens on-chain.

**Capabilities:**

- Mint tokens against a verified Project ID with a 1:1 correspondence to Registry-verified Credits
- Enforce one-to-one backing — prevents over-issuance
- Transfer tokens between Wallets
- Burn (retire) tokens permanently — retired tokens cannot be transferred, re-listed, or re-minted
- Emit `mint` and `retirement` events with full metadata

### Marketplace

Handles listing creation, order matching, and atomic settlement.

**Capabilities:**

- Create listings with Token ID, quantity, and asking price
- Validate Seller balance before listing
- Lock listed tokens in escrow
- Match Buyer orders to listings — atomic token-for-payment settlement
- Partial fill support — auto-close listing when quantity reaches zero
- Allow Seller to cancel listing before match (releases escrow)

### Escrow

Holds tokens securely during active listings.

**Capabilities:**

- Lock tokens from Seller's wallet on listing creation
- Release tokens back to Seller on cancellation
- Transfer tokens to Buyer on settlement
- Prevent any other movement of escrowed tokens

### VerificationRecords

On-chain registry of verified carbon credit projects linking tokens to real-world certificates.

**Capabilities:**

- Create immutable Verification_Records linking Registry name, certificate ID, Project ID, and timestamp
- Suspend tokens associated with a revoked certificate
- Reinstate previously suspended projects when certificates are restored
- Query project metadata by Token ID

### AuditLog

Append-only, tamper-evident log of all state-changing events.

**Capabilities:**

- Append-only writes — rejects modifications and deletions of existing entries
- Merkle-tree hash chain for cryptographic integrity verification
- Records: registrations, minting, listings, orders, settlements, retirements, certificate revocations, upgrade objections

### Governance

Time-locked proxy upgrade system for smart contract administration.

**Capabilities:**

- Minimum 48-hour delay between upgrade proposal and execution
- Multi-signature authorization (minimum 3 designated administrators)
- Publish proposed changes and execution timestamp on-chain
- Record objections from Token holders in AuditLog
- Emit upgrade event and notify Issuers

## Architecture

```
                      ┌──────────────────────┐
                      │       Governance      │
                      │  (Upgrade + Multi-sig)│
                      └──────────┬───────────┘
                                 │
   ┌────────────┐     ┌─────────▼──────────┐     ┌─────────────┐
   │CarbonCredit│◄────┤     Marketplace     │────►│   Escrow    │
   │   Token    │     │  (Listing + Order)  │     │  (Lockbox)  │
   └─────┬──────┘     └────────────────────┘     └─────────────┘
         │                      │
         │              ┌───────▼────────┐
         │              │  Verification  │
         ├──────────────┤    Records     │
         │              └───────┬────────┘
         │                      │
   ┌─────▼──────────────────────▼─────────┐
   │              AuditLog                 │
   │   (Append-only, Merkle-chain)         │
   └──────────────────────────────────────┘
```

All contracts share common Soroban types, error definitions, and authorization patterns.

## Tech Stack

- **Language:** Rust (edition 2021)
- **SDK:** Soroban SDK `soroban-sdk`
- **Testing:** `soroban-sdk` test harness with `#[test]`
- **Build:** Cargo with workspace
- **Deployment:** Soroban CLI / Stellar RPC

## Project Structure

```
contracts/
├── Cargo.toml              # Workspace manifest
├── contracts/
│   ├── carbon-token/       # CarbonCreditToken contract
│   ├── marketplace/        # Marketplace contract
│   ├── escrow/             # Escrow lockbox contract
│   ├── verification-records/ # On-chain registry records
│   ├── audit-log/          # Append-only audit log
│   └── governance/         # Upgrade + multi-sig governance
├── common/                 # Shared types, errors, auth utilities
├── scripts/
│   ├── deploy.sh           # Deployment scripts
│   ├── release.sh          # Production release with validation
│   ├── upgrade.sh          # Governance upgrade helper
│   └── verify.sh           # Verification/migration scripts
└── tests/
    ├── integration/        # Cross-contract integration tests
    └── fixtures/           # Test data / mock registries
```

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.77+
- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup#install-the-soroban-cli)
- Target `wasm32-unknown-unknown`: `rustup target add wasm32-unknown-unknown`
- Stellar testnet account with friendbot-funding

## Getting Started

```bash
# Clone the repository
git clone https://github.com/mintcarbon/mintcarbon-contracts.git
cd mintcarbon-contracts

# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Run unit tests
cargo test

# Run integration tests
cargo test --package tests

# Deploy contracts to testnet
./scripts/deploy.sh --network testnet
```

## Community and Contributing

We welcome contributions from the community! Please see the following documents for more information:

- [Contributing Guidelines](CONTRIBUTING.md) - How to get started with development.
- [Code of Conduct](CODE_OF_CONDUCT.md) - Our standards for a welcoming environment.
- [Security Policy](SECURITY.md) - How to report security vulnerabilities.

## Key Design Decisions

1. **Proxy upgrade pattern** — `Governance` contract acts as a proxy registry, delegating to implementation contracts. Upgrades require multi-sig + 48h timelock.

2. **Merkle-chain audit log** — Each entry's hash includes the previous entry's hash, forming a chain. Periodic Merkle roots are stored on-chain for efficient verification.

3. **Escrow-first listing** — Tokens are moved to the Escrow contract at listing time, preventing double-listing or transfer of active listings.

4. **Atomic settlement** — Order matching is a single Soroban invocation that transfers tokens from escrow to buyer + transfers native asset/XLM from buyer to seller, all-or-nothing.

## Event Reference

| Event                 | Emitter             | Payload                                                            |
| --------------------- | ------------------- | ------------------------------------------------------------------ |
| `mint`                | CarbonCreditToken   | token_id, quantity, project_id, verification_record_ref, timestamp |
| `retire`              | CarbonCreditToken   | token_id, quantity, wallet, reason, timestamp                      |
| `listing_created`     | Marketplace         | seller, token_id, quantity, price, listing_id, timestamp           |
| `order_matched`       | Marketplace         | buyer, seller, token_id, quantity, unit_price, total, tx_hash      |
| `listing_cancelled`   | Marketplace         | seller, listing_id, timestamp                                      |
| `escrow_locked`       | Escrow              | listing_id, seller, token_id, quantity                             |
| `escrow_released`     | Escrow              | listing_id, seller, token_id, quantity                             |
| `escrow_settled`      | Escrow              | listing_id, buyer, token_id, quantity                              |
| `certificate_revoked` | VerificationRecords | registry, cert_id, project_id, timestamp                           |
| `reinstate`           | VerificationRecords | project_id                                                         |
| `upgrade_proposed`    | Governance          | new_impl, scheduled_time, proposer                                 |
| `upgrade_executed`    | Governance          | new_impl, timestamp                                                |

## Configuration

Configuration is environment-specific via Soroban CLI profiles:

| Variable        | Description                     | Default        |
| --------------- | ------------------------------- | -------------- |
| `NETWORK`       | Stellar network target          | `testnet`      |
| `ADMIN_KEYS`    | Multi-sig admin public keys     | —              |
| `TIMELOCK_SECS` | Governance timelock duration    | `172800` (48h) |
| `MIN_SIGS`      | Required signatures for upgrade | `3`            |

## Security Considerations

- All state-changing functions use Soroban's `require_auth()` for access control
- Multi-sig threshold prevents single-administrator compromise
- Timelock gives Token holders opportunity to review and object to upgrades
- Reentrancy protection via checks-effects-interactions pattern
- Escrow contract has no publicly callable withdraw — only Marketplace can trigger releases

## License

This project is licensed under the [MIT License](LICENSE).
