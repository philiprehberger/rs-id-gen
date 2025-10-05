# rs-id-gen

[![CI](https://github.com/philiprehberger/rs-id-gen/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-id-gen/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-id-gen.svg)](https://crates.io/crates/philiprehberger-id-gen)
[![License](https://img.shields.io/github/license/philiprehberger/rs-id-gen)](LICENSE)
[![Sponsor](https://img.shields.io/badge/sponsor-GitHub%20Sponsors-ec6cb9)](https://github.com/sponsors/philiprehberger)

Unified ID generation: ULID, UUIDv7, NanoID, and Snowflake

## Installation

```toml
[dependencies]
philiprehberger-id-gen = "0.2.2"
```

## Usage

```rust
use philiprehberger_id_gen::{Ulid, Uuid7, NanoId, SnowflakeGenerator};

// ULID — time-sortable, Crockford base32
let id = Ulid::new();
println!("{}", id); // e.g. "01ARZ3NDEKTSV4RRFFQ69G5FAV"

// UUIDv7 — time-sortable UUID
let id = Uuid7::new();
println!("{}", id); // e.g. "018e4f5c-8b9a-7d3e-a456-426614174000"

// NanoID — compact, URL-safe
let id = NanoId::new();
println!("{}", id); // e.g. "V1StGXR8_Z5jdHi6B-myT"

// Snowflake — distributed sequential IDs
let gen = SnowflakeGenerator::new(1);
let id = gen.next_id();
println!("{}", id); // e.g. "6820873600000004097"
```

### Serde support

```toml
[dependencies]
philiprehberger-id-gen = { version = "0.2.2", features = ["serde"] }
```

## API

| Type | Description |
|------|-------------|
| `Ulid::new()` | Generate a ULID |
| `Uuid7::new()` | Generate a UUIDv7 |
| `NanoId::new()` | Generate a 21-char NanoID |
| `NanoId::with_alphabet(alphabet, len)` | NanoID with custom alphabet |
| `SnowflakeGenerator::new(machine_id)` | Create a Snowflake generator |
| `SnowflakeGenerator::with_epoch(machine_id, epoch)` | Custom epoch |
| `.next_id()` | Generate next Snowflake ID |
| `Snowflake::value()` | Get raw u64 value (const fn) |
| `NanoId::as_ref()` | Get &str reference |
| `From<u64> for Snowflake` | Construct from raw value |


## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## License

MIT
