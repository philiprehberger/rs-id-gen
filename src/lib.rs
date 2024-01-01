//! Unified ID generation: ULID, UUIDv7, NanoID, and Snowflake.
//!
//! This crate provides four popular ID generation algorithms with zero external
//! dependencies. All randomness is sourced from a thread-local xorshift64 RNG
//! seeded from `SystemTime`.
//!
//! # Examples
//!
//! ```
//! use philiprehberger_id_gen::{Ulid, Uuid7, NanoId, SnowflakeGenerator};
//!
//! let ulid = Ulid::new();
//! println!("ULID: {}", ulid);
//!
//! let uuid = Uuid7::new();
//! println!("UUIDv7: {}", uuid);
//!
//! let nano = NanoId::new();
//! println!("NanoID: {}", nano);
//!
//! let gen = SnowflakeGenerator::new(1);
//! let sf = gen.next_id();
//! println!("Snowflake: {}", sf);
//! ```

use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Thread-local xorshift64 RNG
// ---------------------------------------------------------------------------

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        // Ensure state is never zero
        let state = if seed == 0 { 0xDEAD_BEEF_CAFE_BABE } else { seed };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

thread_local! {
    static RNG: RefCell<Xorshift64> = RefCell::new({
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        // Mix in the thread ID for uniqueness across threads
        let tid = std::thread::current().id();
        let tid_hash = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            tid.hash(&mut h);
            h.finish()
        };
        Xorshift64::new(seed ^ tid_hash)
    });
}

fn rand_u64() -> u64 {
    RNG.with(|rng| rng.borrow_mut().next_u64())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// ULID
// ---------------------------------------------------------------------------

/// A Universally Unique Lexicographically Sortable Identifier (ULID).
///
/// 128 bits: 48-bit timestamp (milliseconds since Unix epoch) + 80-bit random.
/// Encoded as 26-character Crockford base32 string.
///
/// ULIDs generated within the same millisecond are monotonically ordered by
/// incrementing the random component.
///
/// # Examples
///
/// ```
/// use philiprehberger_id_gen::Ulid;
///
/// let id = Ulid::new();
/// let s = id.to_string();
/// assert_eq!(s.len(), 26);
///
/// let parsed: Ulid = s.parse().unwrap();
/// assert_eq!(id, parsed);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Ulid {
    /// Most significant 64 bits: 48-bit timestamp in upper bits + 16 bits of randomness.
    msb: u64,
    /// Least significant 64 bits: remaining 64 bits of randomness.
    lsb: u64,
}

// Monotonic state for ULID
thread_local! {
    static ULID_LAST_MS: RefCell<u64> = const { RefCell::new(0) };
    static ULID_LAST_RANDOM_MSB: RefCell<u16> = const { RefCell::new(0) };
    static ULID_LAST_RANDOM_LSB: RefCell<u64> = const { RefCell::new(0) };
}

const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn crockford_decode_char(c: u8) -> Option<u8> {
    match c {
        b'0' | b'O' | b'o' => Some(0),
        b'1' | b'I' | b'i' | b'L' | b'l' => Some(1),
        b'2' => Some(2),
        b'3' => Some(3),
        b'4' => Some(4),
        b'5' => Some(5),
        b'6' => Some(6),
        b'7' => Some(7),
        b'8' => Some(8),
        b'9' => Some(9),
        b'A' | b'a' => Some(10),
        b'B' | b'b' => Some(11),
        b'C' | b'c' => Some(12),
        b'D' | b'd' => Some(13),
        b'E' | b'e' => Some(14),
        b'F' | b'f' => Some(15),
        b'G' | b'g' => Some(16),
        b'H' | b'h' => Some(17),
        b'J' | b'j' => Some(18),
        b'K' | b'k' => Some(19),
        b'M' | b'm' => Some(20),
        b'N' | b'n' => Some(21),
        b'P' | b'p' => Some(22),
        b'Q' | b'q' => Some(23),
        b'R' | b'r' => Some(24),
        b'S' | b's' => Some(25),
        b'T' | b't' => Some(26),
        b'V' | b'v' => Some(27),
        b'W' | b'w' => Some(28),
        b'X' | b'x' => Some(29),
        b'Y' | b'y' => Some(30),
        b'Z' | b'z' => Some(31),
        _ => None,
    }
}

impl Ulid {
    /// Generates a new ULID with the current timestamp.
    ///
    /// ULIDs generated within the same millisecond are monotonically ordered.
    pub fn new() -> Self {
        let ms = now_millis();

        ULID_LAST_MS.with(|last_ms_cell| {
            ULID_LAST_RANDOM_MSB.with(|last_rmsb_cell| {
                ULID_LAST_RANDOM_LSB.with(|last_rlsb_cell| {
                    let mut last_ms = last_ms_cell.borrow_mut();
                    let mut last_rmsb = last_rmsb_cell.borrow_mut();
                    let mut last_rlsb = last_rlsb_cell.borrow_mut();

                    let (rand_msb_16, rand_lsb_64) = if ms == *last_ms {
                        // Same millisecond: increment random bits for monotonicity
                        let new_lsb = (*last_rlsb).wrapping_add(1);
                        let carry = if new_lsb == 0 { 1u16 } else { 0u16 };
                        let new_msb_16 = (*last_rmsb).wrapping_add(carry);
                        *last_rlsb = new_lsb;
                        *last_rmsb = new_msb_16;
                        (new_msb_16, new_lsb)
                    } else {
                        // New millisecond: fresh random bits
                        let r1 = rand_u64();
                        let r2 = rand_u64();
                        let rmsb = (r1 & 0xFFFF) as u16;
                        let rlsb = r2;
                        *last_ms = ms;
                        *last_rmsb = rmsb;
                        *last_rlsb = rlsb;
                        (rmsb, rlsb)
                    };

                    // msb: 48-bit timestamp | 16-bit random
                    let msb = (ms << 16) | (rand_msb_16 as u64);
                    let lsb = rand_lsb_64;

                    Ulid { msb, lsb }
                })
            })
        })
    }

    /// Returns the timestamp component in milliseconds since Unix epoch.
    pub fn timestamp_ms(&self) -> u64 {
        self.msb >> 16
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ULID is 128 bits encoded as 26 Crockford base32 characters.
        // We treat it as a 128-bit integer: msb (upper 64) | lsb (lower 64).
        let hi = self.msb as u128;
        let lo = self.lsb as u128;
        let value = (hi << 64) | lo;

        let mut buf = [0u8; 26];
        let mut v = value;
        for i in (0..26).rev() {
            buf[i] = CROCKFORD_ALPHABET[(v & 0x1F) as usize];
            v >>= 5;
        }
        let s = std::str::from_utf8(&buf).unwrap();
        f.write_str(s)
    }
}

/// Error returned when parsing an ID from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIdError {
    msg: &'static str,
}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.msg)
    }
}

impl std::error::Error for ParseIdError {}

impl FromStr for Ulid {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 26 {
            return Err(ParseIdError {
                msg: "ULID must be 26 characters",
            });
        }

        let mut value: u128 = 0;
        for &byte in s.as_bytes() {
            let digit = crockford_decode_char(byte).ok_or(ParseIdError {
                msg: "invalid Crockford base32 character",
            })?;
            value = (value << 5) | (digit as u128);
        }

        let msb = (value >> 64) as u64;
        let lsb = value as u64;
        Ok(Ulid { msb, lsb })
    }
}

// ---------------------------------------------------------------------------
// UUIDv7
// ---------------------------------------------------------------------------

/// A UUIDv7 as specified in RFC 9562.
///
/// 128 bits: 48-bit unix_ts_ms | 4-bit version (0x7) | 12-bit rand_a |
/// 2-bit variant (0b10) | 62-bit rand_b.
///
/// # Examples
///
/// ```
/// use philiprehberger_id_gen::Uuid7;
///
/// let id = Uuid7::new();
/// let s = id.to_string();
/// assert_eq!(s.len(), 36);
/// assert_eq!(&s[14..15], "7"); // version nibble
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Uuid7 {
    /// Upper 64 bits of the UUID.
    msb: u64,
    /// Lower 64 bits of the UUID.
    lsb: u64,
}

impl Uuid7 {
    /// Generates a new UUIDv7 with the current timestamp.
    pub fn new() -> Self {
        let ms = now_millis();
        let r1 = rand_u64();
        let r2 = rand_u64();

        // msb: 48-bit timestamp | 4-bit version (0b0111) | 12-bit rand_a
        let rand_a = r1 & 0x0FFF;
        let msb = (ms << 16) | (0x7 << 12) | rand_a;

        // lsb: 2-bit variant (0b10) | 62-bit rand_b
        let rand_b = r2 & 0x3FFF_FFFF_FFFF_FFFF;
        let lsb = (0b10u64 << 62) | rand_b;

        Uuid7 { msb, lsb }
    }

    /// Returns the timestamp component in milliseconds since Unix epoch.
    pub fn timestamp_ms(&self) -> u64 {
        self.msb >> 16
    }
}

impl Default for Uuid7 {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Uuid7 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        // Bytes layout from msb (8 bytes) and lsb (8 bytes):
        // msb bytes: [0..4]-[4..6]-[6..8]  lsb bytes: [0..2]-[2..8]
        let bytes = {
            let mut b = [0u8; 16];
            b[0..8].copy_from_slice(&self.msb.to_be_bytes());
            b[8..16].copy_from_slice(&self.lsb.to_be_bytes());
            b
        };

        let mut buf = [0u8; 36];
        let hex = b"0123456789abcdef";
        let mut pos = 0;
        for (i, &byte) in bytes.iter().enumerate() {
            if i == 4 || i == 6 || i == 8 || i == 10 {
                buf[pos] = b'-';
                pos += 1;
            }
            buf[pos] = hex[(byte >> 4) as usize];
            buf[pos + 1] = hex[(byte & 0x0F) as usize];
            pos += 2;
        }

        let s = std::str::from_utf8(&buf).unwrap();
        f.write_str(s)
    }
}

impl FromStr for Uuid7 {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 36 {
            return Err(ParseIdError {
                msg: "UUID must be 36 characters",
            });
        }

        let mut bytes = [0u8; 16];
        let mut byte_idx = 0;

        let chars: Vec<u8> = s.bytes().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == b'-' {
                i += 1;
                continue;
            }
            if byte_idx >= 16 || i + 1 >= chars.len() {
                return Err(ParseIdError {
                    msg: "invalid UUID format",
                });
            }
            let hi = hex_val(chars[i]).ok_or(ParseIdError {
                msg: "invalid hex character",
            })?;
            let lo = hex_val(chars[i + 1]).ok_or(ParseIdError {
                msg: "invalid hex character",
            })?;
            bytes[byte_idx] = (hi << 4) | lo;
            byte_idx += 1;
            i += 2;
        }

        if byte_idx != 16 {
            return Err(ParseIdError {
                msg: "invalid UUID format",
            });
        }

        let msb = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let lsb = u64::from_be_bytes(bytes[8..16].try_into().unwrap());

        Ok(Uuid7 { msb, lsb })
    }
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// NanoID
// ---------------------------------------------------------------------------

const NANOID_DEFAULT_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
const NANOID_DEFAULT_LEN: usize = 21;

/// A NanoID — a compact, URL-friendly unique string identifier.
///
/// Default: 21 characters from the alphabet `A-Za-z0-9_-`.
///
/// # Examples
///
/// ```
/// use philiprehberger_id_gen::NanoId;
///
/// let id = NanoId::new();
/// assert_eq!(id.as_str().len(), 21);
///
/// let custom = NanoId::with_alphabet("abc123", 10);
/// assert_eq!(custom.as_str().len(), 10);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NanoId {
    value: String,
}

impl NanoId {
    /// Generates a new NanoID with the default alphabet (`A-Za-z0-9_-`) and length 21.
    pub fn new() -> Self {
        Self::with_alphabet(
            std::str::from_utf8(NANOID_DEFAULT_ALPHABET).unwrap(),
            NANOID_DEFAULT_LEN,
        )
    }

    /// Generates a NanoID with a custom alphabet and length.
    ///
    /// # Panics
    ///
    /// Panics if `alphabet` is empty.
    pub fn with_alphabet(alphabet: &str, len: usize) -> Self {
        assert!(!alphabet.is_empty(), "alphabet must not be empty");

        let alpha_bytes = alphabet.as_bytes();
        let alpha_len = alpha_bytes.len();

        // Use bitmask technique to reduce modulo bias
        let mask = (1usize << (usize::BITS - alpha_len.leading_zeros())) - 1;

        let mut result = String::with_capacity(len);
        while result.len() < len {
            let r = rand_u64();
            // Extract multiple indices from a single random u64
            let mut bits = r;
            for _ in 0..8 {
                if result.len() >= len {
                    break;
                }
                let idx = (bits as usize) & mask;
                if idx < alpha_len {
                    result.push(alpha_bytes[idx] as char);
                }
                bits >>= 8;
            }
        }

        NanoId { value: result }
    }

    /// Returns the NanoID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Default for NanoId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NanoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl FromStr for NanoId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseIdError {
                msg: "NanoId cannot be empty",
            });
        }
        Ok(NanoId {
            value: s.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Snowflake
// ---------------------------------------------------------------------------

/// Default Snowflake epoch: 2020-01-01T00:00:00Z (1577836800000 ms since Unix epoch).
const DEFAULT_SNOWFLAKE_EPOCH: u64 = 1_577_836_800_000;

/// A Snowflake ID generator with configurable machine ID and epoch.
///
/// Produces 64-bit IDs: 41-bit timestamp + 10-bit machine ID + 12-bit sequence.
/// The timestamp is milliseconds since the configured epoch.
///
/// # Examples
///
/// ```
/// use philiprehberger_id_gen::SnowflakeGenerator;
///
/// let gen = SnowflakeGenerator::new(1);
/// let id1 = gen.next_id();
/// let id2 = gen.next_id();
/// assert!(id2 > id1);
/// ```
pub struct SnowflakeGenerator {
    machine_id: u16,
    epoch_ms: u64,
    /// Packed atomic state: upper 52 bits = last timestamp, lower 12 bits = sequence.
    state: AtomicU64,
}

/// A generated Snowflake ID (64-bit).
///
/// Layout: 1 unused sign bit | 41-bit timestamp | 10-bit machine ID | 12-bit sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Snowflake(u64);

impl SnowflakeGenerator {
    /// Creates a new Snowflake generator with the given machine ID.
    ///
    /// The machine ID must fit in 10 bits (0..1023).
    ///
    /// # Panics
    ///
    /// Panics if `machine_id` exceeds 1023.
    pub fn new(machine_id: u16) -> Self {
        Self::with_epoch(machine_id, DEFAULT_SNOWFLAKE_EPOCH)
    }

    /// Creates a new Snowflake generator with a custom epoch (milliseconds since Unix epoch).
    ///
    /// # Panics
    ///
    /// Panics if `machine_id` exceeds 1023.
    pub fn with_epoch(machine_id: u16, epoch_ms: u64) -> Self {
        assert!(
            machine_id < 1024,
            "machine_id must fit in 10 bits (0..1023)"
        );
        SnowflakeGenerator {
            machine_id,
            epoch_ms,
            state: AtomicU64::new(0),
        }
    }

    /// Generates the next Snowflake ID.
    ///
    /// IDs are monotonically increasing. If the sequence overflows within the same
    /// millisecond, this method busy-waits until the next millisecond.
    pub fn next_id(&self) -> Snowflake {
        loop {
            let now = now_millis().saturating_sub(self.epoch_ms);
            let current = self.state.load(Ordering::Relaxed);
            let last_ts = current >> 12;
            let seq = current & 0xFFF;

            if now == last_ts {
                let new_seq = seq + 1;
                if new_seq > 0xFFF {
                    // Sequence overflow, busy-wait for next ms
                    std::hint::spin_loop();
                    continue;
                }
                let new_state = (now << 12) | new_seq;
                if self
                    .state
                    .compare_exchange(current, new_state, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    let id =
                        (now << 22) | ((self.machine_id as u64) << 12) | new_seq;
                    return Snowflake(id);
                }
                // CAS failed, retry
                continue;
            }

            // New millisecond — reset sequence to 0
            let new_state = now << 12;
            if self
                .state
                .compare_exchange(current, new_state, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                let id = (now << 22) | ((self.machine_id as u64) << 12);
                return Snowflake(id);
            }
            // CAS failed, retry
        }
    }
}

impl Snowflake {
    /// Returns the raw 64-bit value.
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Extracts the timestamp component (milliseconds since the generator's epoch).
    pub fn timestamp(&self) -> u64 {
        self.0 >> 22
    }

    /// Extracts the machine ID component.
    pub fn machine_id(&self) -> u16 {
        ((self.0 >> 12) & 0x3FF) as u16
    }

    /// Extracts the sequence number component.
    pub fn sequence(&self) -> u16 {
        (self.0 & 0xFFF) as u16
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Snowflake {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: u64 = s.parse().map_err(|_| ParseIdError {
            msg: "invalid Snowflake ID",
        })?;
        Ok(Snowflake(v))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ULID tests --

    #[test]
    fn ulid_uniqueness() {
        let ids: Vec<Ulid> = (0..100).map(|_| Ulid::new()).collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "ULIDs should be unique");
            }
        }
    }

    #[test]
    fn ulid_monotonic_ordering() {
        let ids: Vec<Ulid> = (0..100).map(|_| Ulid::new()).collect();
        for i in 1..ids.len() {
            assert!(
                ids[i] > ids[i - 1],
                "ULIDs should be monotonically ordered"
            );
        }
    }

    #[test]
    fn ulid_display_length() {
        let id = Ulid::new();
        let s = id.to_string();
        assert_eq!(s.len(), 26, "ULID string should be 26 characters");
    }

    #[test]
    fn ulid_fromstr_roundtrip() {
        let id = Ulid::new();
        let s = id.to_string();
        let parsed: Ulid = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn ulid_invalid_parse() {
        assert!("too_short".parse::<Ulid>().is_err());
        assert!("!!!!!!!!!!!!!!!!!!!!!!!!!!".parse::<Ulid>().is_err());
    }

    // -- UUIDv7 tests --

    #[test]
    fn uuid7_format() {
        let id = Uuid7::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36);
        // Check hyphens at correct positions
        assert_eq!(s.as_bytes()[8], b'-');
        assert_eq!(s.as_bytes()[13], b'-');
        assert_eq!(s.as_bytes()[18], b'-');
        assert_eq!(s.as_bytes()[23], b'-');
    }

    #[test]
    fn uuid7_version_bits() {
        let id = Uuid7::new();
        let s = id.to_string();
        // Version nibble at position 14 should be '7'
        assert_eq!(&s[14..15], "7");
    }

    #[test]
    fn uuid7_variant_bits() {
        let id = Uuid7::new();
        // Variant is the top 2 bits of lsb, should be 0b10
        let variant = (id.lsb >> 62) & 0b11;
        assert_eq!(variant, 0b10, "variant bits should be 0b10");
    }

    #[test]
    fn uuid7_fromstr_roundtrip() {
        let id = Uuid7::new();
        let s = id.to_string();
        let parsed: Uuid7 = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn uuid7_uniqueness() {
        let ids: Vec<Uuid7> = (0..100).map(|_| Uuid7::new()).collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j]);
            }
        }
    }

    // -- NanoID tests --

    #[test]
    fn nanoid_default_length() {
        let id = NanoId::new();
        assert_eq!(id.as_str().len(), 21);
    }

    #[test]
    fn nanoid_default_alphabet() {
        let id = NanoId::new();
        let valid: &[u8] = NANOID_DEFAULT_ALPHABET;
        for c in id.as_str().bytes() {
            assert!(
                valid.contains(&c),
                "character '{}' not in default alphabet",
                c as char
            );
        }
    }

    #[test]
    fn nanoid_custom_alphabet_and_length() {
        let id = NanoId::with_alphabet("abc", 10);
        assert_eq!(id.as_str().len(), 10);
        for c in id.as_str().chars() {
            assert!(
                "abc".contains(c),
                "character '{}' not in custom alphabet",
                c
            );
        }
    }

    #[test]
    fn nanoid_fromstr_roundtrip() {
        let id = NanoId::new();
        let s = id.to_string();
        let parsed: NanoId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    #[should_panic(expected = "alphabet must not be empty")]
    fn nanoid_empty_alphabet_panics() {
        NanoId::with_alphabet("", 10);
    }

    // -- Snowflake tests --

    #[test]
    fn snowflake_ordering() {
        let gen = SnowflakeGenerator::new(1);
        let ids: Vec<Snowflake> = (0..100).map(|_| gen.next_id()).collect();
        for i in 1..ids.len() {
            assert!(
                ids[i] > ids[i - 1],
                "Snowflake IDs should be monotonically increasing"
            );
        }
    }

    #[test]
    fn snowflake_machine_id_extraction() {
        let gen = SnowflakeGenerator::new(42);
        let id = gen.next_id();
        assert_eq!(id.machine_id(), 42);
    }

    #[test]
    fn snowflake_fromstr_roundtrip() {
        let gen = SnowflakeGenerator::new(7);
        let id = gen.next_id();
        let s = id.to_string();
        let parsed: Snowflake = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn snowflake_components() {
        let gen = SnowflakeGenerator::new(100);
        let id = gen.next_id();
        assert_eq!(id.machine_id(), 100);
        assert!(id.timestamp() > 0);
        assert_eq!(id.sequence(), 0); // first ID in a new ms should have seq 0
    }

    #[test]
    #[should_panic(expected = "machine_id must fit in 10 bits")]
    fn snowflake_invalid_machine_id() {
        SnowflakeGenerator::new(1024);
    }

    #[test]
    fn snowflake_custom_epoch() {
        let epoch = 1_700_000_000_000; // some custom epoch
        let gen = SnowflakeGenerator::with_epoch(1, epoch);
        let id = gen.next_id();
        // Timestamp should be relative to custom epoch, so much smaller
        let ts = id.timestamp();
        assert!(ts < now_millis()); // sanity check: relative ts < absolute ts
    }
}
