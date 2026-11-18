//! Little-endian LEB128 run-length coding for grids and bit sets.

use crate::SaveError;

fn put(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn take(bytes: &[u8], at: &mut usize) -> Result<u64, SaveError> {
    let mut value = 0_u64;
    for shift in (0..70).step_by(7) {
        let byte = *bytes
            .get(*at)
            .ok_or(SaveError::Corrupt("truncated run-length data"))?;
        *at += 1;
        if shift == 63 && byte > 1 {
            return Err(SaveError::Corrupt("run-length value overflows"));
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(SaveError::Corrupt("run-length value overflows"))
}

/// Encodes values as (value, run length) pairs.
pub fn encode_runs(values: impl IntoIterator<Item = u32>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut current: Option<(u32, u64)> = None;
    for value in values {
        match &mut current {
            Some((v, run)) if *v == value => *run += 1,
            _ => {
                if let Some((v, run)) = current.replace((value, 1)) {
                    put(&mut out, u64::from(v));
                    put(&mut out, run);
                }
            }
        }
    }
    if let Some((v, run)) = current {
        put(&mut out, u64::from(v));
        put(&mut out, run);
    }
    out
}

/// Decodes `expected` values; the data must hold exactly that many.
pub fn decode_runs(bytes: &[u8], expected: usize) -> Result<Vec<u32>, SaveError> {
    let mut out = Vec::with_capacity(expected);
    let mut at = 0;
    while at < bytes.len() {
        let value = u32::try_from(take(bytes, &mut at)?)
            .map_err(|_| SaveError::Corrupt("grid value out of range"))?;
        let run = usize::try_from(take(bytes, &mut at)?)
            .map_err(|_| SaveError::Corrupt("grid run out of range"))?;
        if run == 0 || out.len() + run > expected {
            return Err(SaveError::Corrupt("grid run length"));
        }
        out.resize(out.len() + run, value);
    }
    if out.len() != expected {
        return Err(SaveError::Corrupt("grid length"));
    }
    Ok(out)
}

/// Words of a bit set as (word, run length) pairs (format 12, task 99: the fog of system
/// space is mostly empty); no words give no bytes.
pub fn encode_words(words: &[u64]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < words.len() {
        let run = words[at..].iter().take_while(|w| **w == words[at]).count();
        put(&mut out, words[at]);
        put(&mut out, run as u64);
        at += run;
    }
    out
}

/// Run-length pairs as words; the caller checks the count.
pub fn decode_words(bytes: &[u8]) -> Result<Vec<u64>, SaveError> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let word = take(bytes, &mut at)?;
        let run = usize::try_from(take(bytes, &mut at)?)
            .map_err(|_| SaveError::Corrupt("bit set run out of range"))?;
        if run == 0 || run > (1 << 26) - out.len() {
            return Err(SaveError::Corrupt("bit set run length"));
        }
        out.resize(out.len() + run, word);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_round_trip_and_reject_damage() {
        let values: Vec<u32> =
            [vec![7; 1000], vec![0; 3], vec![u32::MAX], vec![5; 70_000]].concat();
        let bytes = encode_runs(values.iter().copied());
        assert!(bytes.len() < 20);
        assert_eq!(decode_runs(&bytes, values.len()).unwrap(), values);
        assert!(decode_runs(&bytes, values.len() + 1).is_err());
        assert!(decode_runs(&bytes[..bytes.len() - 1], values.len()).is_err());
        assert_eq!(decode_runs(&[], 0).unwrap(), Vec::<u32>::new());
    }

    #[test]
    fn words_round_trip() {
        let words = [vec![1, u64::MAX, 0x0123_4567_89ab_cdef], vec![0; 5000]].concat();
        let bytes = encode_words(&words);
        assert!(bytes.len() < 32);
        assert_eq!(decode_words(&bytes).unwrap(), words);
        assert_eq!(decode_words(&[]).unwrap(), Vec::<u64>::new());
        assert!(decode_words(&bytes[..bytes.len() - 1]).is_err());
        assert!(decode_words(&[1, 0]).is_err());
    }
}
