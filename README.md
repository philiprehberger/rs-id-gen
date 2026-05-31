# rs-id-gen

[![CI](https://github.com/philiprehberger/rs-id-gen/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-id-gen/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-id-gen.svg)](https://crates.io/crates/philiprehberger-id-gen)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-id-gen)](https://github.com/philiprehberger/rs-id-gen/commits/main)

![rs-id-gen](https://raw.githubusercontent.com/philiprehberger/rs-id-gen/main/package-card.webp)

Unified ID generation: ULID, UUIDv7, NanoID, and Snowflake

## Installation

```toml
[dependencies]
philiprehberger-id-gen = "0.3.0"
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
philiprehberger-id-gen = { version = "0.3.0", features = ["serde"] }
```

### Binary interop and timestamp reconstruction

```rust
use philiprehberger_id_gen::{Ulid, Uuid7, Snowflake};

// ULID ↔ 16-byte big-endian
let id = Ulid::new();
let bytes: [u8; 16] = id.to_bytes();
let restored = Ulid::from_bytes(bytes);
assert_eq!(id, restored);

// Reconstruct a ULID with a specific timestamp (e.g. for tests or backfills)
let backfill = Ulid::from_timestamp_ms(1_700_000_000_000);
assert_eq!(backfill.timestamp_ms(), 1_700_000_000_000);

// Construct a Snowflake from a stored u64
let sf = Snowflake::from_value(123_456_789);
assert_eq!(sf.value(), 123_456_789);

// Same patterns work for UUIDv7
let u = Uuid7::new();
let restored = Uuid7::from_bytes(u.to_bytes());
assert_eq!(u, restored);
```

## API

| Type | Description |
|------|-------------|
| `Ulid::new()` | Generate a ULID |
| `Ulid::from_timestamp_ms(ms)` | ULID with a specific timestamp + fresh randomness |
| `Ulid::to_bytes()` | 16-byte big-endian representation |
| `Ulid::from_bytes(bytes)` | Reconstruct from 16-byte big-endian |
| `Uuid7::new()` | Generate a UUIDv7 |
| `Uuid7::from_timestamp_ms(ms)` | UUIDv7 with a specific timestamp + fresh randomness |
| `Uuid7::to_bytes()` | 16-byte big-endian representation |
| `Uuid7::from_bytes(bytes)` | Reconstruct from 16-byte big-endian |
| `NanoId::new()` | Generate a 21-char NanoID |
| `NanoId::with_alphabet(alphabet, len)` | NanoID with custom alphabet |
| `SnowflakeGenerator::new(machine_id)` | Create a Snowflake generator |
| `SnowflakeGenerator::with_epoch(machine_id, epoch)` | Custom epoch |
| `.next_id()` | Generate next Snowflake ID |
| `Snowflake::from_value(value)` | Construct a `Snowflake` from a raw `u64` (const fn) |
| `Snowflake::value()` | Get raw u64 value (const fn) |
| `NanoId::as_ref()` | Get &str reference |
| `From<u64> for Snowflake` | Construct from raw value |

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Support

If you find this project useful:

⭐ [Star the repo](https://github.com/philiprehberger/rs-id-gen)

🐛 [Report issues](https://github.com/philiprehberger/rs-id-gen/issues?q=is%3Aissue+is%3Aopen+label%3Abug)

💡 [Suggest features](https://github.com/philiprehberger/rs-id-gen/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)

❤️ [Sponsor development](https://github.com/sponsors/philiprehberger)

🌐 [All Open Source Projects](https://philiprehberger.com/open-source-packages)

💻 [GitHub Profile](https://github.com/philiprehberger)

🔗 [LinkedIn Profile](https://www.linkedin.com/in/philiprehberger)

## License

[MIT](LICENSE)
