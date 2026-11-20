//! One-time keys and seat tokens (`app/docs/network.md`, "Keys and seats"): random bytes from
//! the operating system, shown in base32, stored only as SHA-256 digests.

use sha2::{Digest, Sha256};

use crate::NetError;

/// Random bytes in a key.
pub const KEY_BYTES: usize = 20;
/// Random bytes in a seat token.
pub const TOKEN_BYTES: usize = 30;

/// RFC 4648 base32 alphabet.
const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// `n` random bytes from the operating system.
fn random(n: usize) -> Result<Vec<u8>, NetError> {
    let mut bytes = vec![0; n];
    getrandom::fill(&mut bytes).map_err(|e| NetError::Random(e.to_string()))?;
    Ok(bytes)
}

/// Base32 without padding.
fn base32(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let (mut buffer, mut bits) = (0_u32, 0_u32);
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

/// A new one-time key: 20 random bytes as 32 base32 characters in four groups of eight,
/// `XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX`.
pub fn new_key() -> Result<String, NetError> {
    let text = base32(&random(KEY_BYTES)?);
    let groups: Vec<&str> = (0..text.len())
        .step_by(8)
        .map(|at| &text[at..(at + 8).min(text.len())])
        .collect();
    Ok(groups.join("-"))
}

/// A new seat token: 30 random bytes as 48 base32 characters.
pub fn new_token() -> Result<String, NetError> {
    Ok(base32(&random(TOKEN_BYTES)?))
}

/// A key or token as typed: upper case, without dashes or spaces.
pub fn normalise(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// The SHA-256 digest, in hex, of a key or token as [`normalise`]d: what the server stores.
pub fn digest(text: &str) -> String {
    let hash = Sha256::digest(normalise(text).as_bytes());
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_follows_rfc_4648() {
        // The RFC's test vectors, without padding.
        assert_eq!(base32(b""), "");
        assert_eq!(base32(b"f"), "MY");
        assert_eq!(base32(b"fo"), "MZXQ");
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn keys_and_tokens_have_their_shape_and_differ() {
        let key = new_key().unwrap();
        assert_eq!(key.len(), 35, "{key}");
        assert_eq!(key.matches('-').count(), 3);
        assert!(
            key.chars()
                .all(|c| c == '-' || ALPHABET.contains(&(c as u8)))
        );
        assert_ne!(new_key().unwrap(), key);
        assert_eq!(new_token().unwrap().len(), 48);
    }

    #[test]
    fn a_key_typed_loosely_has_the_same_digest() {
        let key = "ABCDEFGH-IJKLMNOP-QRSTUVWX-YZ234567";
        assert_eq!(digest(key), digest(" abcdefgh ijklmnop-qrstuvwx yz234567 "));
        assert_eq!(digest(key).len(), 64);
        assert_ne!(digest(key), digest("ABCDEFGH"));
        // SHA-256 of "ABC".
        assert_eq!(
            digest("abc"),
            "b5d4045c3f466fa91fe2cc6abe79232a1a57cdf104f7a26e716e0a1e2789df78"
        );
    }
}
