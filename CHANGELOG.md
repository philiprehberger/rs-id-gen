# Changelog

## 0.2.4 (2026-03-31)

- Standardize README to 3-badge format with emoji Support section
- Update CI checkout action to v5 for Node.js 24 compatibility

## 0.2.3 (2026-03-27)

- Add GitHub issue templates, PR template, and dependabot configuration
- Update README badges and add Support section

## 0.2.2 (2026-03-22)

- Fix stale version in serde usage snippet

## 0.2.1 (2026-03-22)

- Fix CHANGELOG formatting

## 0.2.0 (2026-03-21)

- Add const fn for Snowflake accessor methods (value, timestamp, machine_id, sequence)
- Add AsRef<str> implementation for NanoId
- Add optional serde feature with Serialize/Deserialize for all ID types
- Add From<u64> implementation for Snowflake
- Add #[must_use] attributes on ID generation methods

## 0.1.6 (2026-03-17)

- Add readme, rust-version, documentation to Cargo.toml
- Add Development section to README

## 0.1.5 (2026-03-16)

- Update install snippet to use full version

## 0.1.4 (2026-03-16)

- Add README badges
- Synchronize version across Cargo.toml, README, and CHANGELOG

## 0.1.0 (2026-03-15)

- Initial release
- ULID generation with Crockford base32 encoding
- UUIDv7 generation per RFC 9562
- NanoID generation with configurable alphabet and length
- Snowflake ID generation with configurable machine ID and epoch
