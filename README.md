# Description

A tiny Rust proc-macro crate that enforces explicit, opt-in public APIs. Attach #[access_control] to an *impl* (or an inline mod) and every public function inside must be explicitly marked with #[no_access_control]. This repo also includes a minimal Soroban contract and tests demonstrating the macro in action.

## Installation

Stellar smart contracts are written in the [Rust](https://www.rust-lang.org/) programming language and can be deployed to the testnet or mainnet.

## Prerequisites

To build and develop contracts you need only a couple prerequisites:

- A [Rust](https://www.rust-lang.org/) toolchain
- An editor that supports Rust
- [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup#install-the-stellar-cli)

See the [documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) for more prerequisites installation instructions.

## Build

The stellar contracts can be built with ```make build```.

## Testing

The repository also contains some tests which can be run with ```make test```.
