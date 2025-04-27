# Ticketto Contracts

## Description

This repository contains the smart contracts for the _Events_ module (creating events, configuring event parameters, scheduling event times, and issuing tickets) and the _Tickets_ module (handling attendance logic), in accordance with the Ticketto protocol.

## Folder Structure

```plaintext
.
├── contracts             # Contains ink! smart contracts for the protocol
│   ├── events            # Events module contract
│   └── tickets           # Tickets module contract
└── libs                  # Shared libraries for both contracts
    ├── types             # Crate defining global types used across contracts
    └── traits            # Crate implementing common traits and interfaces
```

## Build Instructions

To compile the `ticketto_events` contracts, run:

```bash
cargo contract build --manifest-path contracts/events/Cargo.toml
```

To compile the `ticketto_tickets` contracts, run:

```bash
cargo contract build --manifest-path contracts/tickets/Cargo.toml
```

## Requirements

- Rust (stable toolchain)
- [ink!](https://github.com/paritytech/ink)
- `cargo-contract`

## License

Licensed under the [Apache License 2.0.](./LICENSE)

## Acknowledgments

- The ink! team for their contract framework
- The Rust community for ongoing support and libraries
- Virto Network for providing Kreivo services
