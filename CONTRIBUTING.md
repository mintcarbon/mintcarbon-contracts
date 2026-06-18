# Contributing to mintcarbon

First off, thank you for considering contributing to mintcarbon! It's people like you that make the open-source community such a great place to learn, inspire, and create.

## Code of Conduct

By participating in this project, you are expected to uphold our [Code of Conduct](CODE_OF_CONDUCT.md).

## How Can I Contribute?

### Reporting Bugs

- Use the [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.md).
- Describe the bug in detail, including steps to reproduce.
- Include information about your environment (OS, Rust version, Soroban CLI version).

### Suggesting Enhancements

- Use the [Feature Request Template](.github/ISSUE_TEMPLATE/feature_request.md).
- Explain why this enhancement would be useful.

### Pull Requests

1. Fork the repository.
2. Create a new branch (`git checkout -b feature/amazing-feature`).
3. Make your changes.
4. Run tests and linting (see [Development Setup](#development-setup)).
5. Commit your changes (`git commit -m 'Add some amazing feature'`).
6. Push to the branch (`git push origin feature/amazing-feature`).
7. Open a Pull Request.

## Development Setup

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup#install-the-soroban-cli)
- Target `wasm32-unknown-unknown`:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

### Building the Contracts

To build all contracts in the workspace:

```bash
cargo build --target wasm32-unknown-unknown --release
```

### Running Tests

To run the full test suite:

```bash
cargo test
```

### Linting and Formatting

We use `rustfmt` and `clippy` to maintain code quality. Please run these before submitting a PR:

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

## Project Structure

- `contracts/`: Individual Soroban smart contracts.
- `common/`: Shared types and utilities used across contracts.
- `tests/`: Integration tests for multi-contract flows.
- `scripts/`: Helper scripts for deployment and environment setup.

## License

By contributing, you agree that your contributions will be licensed under its [MIT License](LICENSE).
