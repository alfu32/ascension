//! Little-endian byte encoding for the saved memory of a computer player.

use crate::AiError;

/// Appends fixed-width little-endian values.
#[derive(Debug, Default)]
pub(crate) struct Writer(pub Vec<u8>);

impl Writer {
    pub fn u8(&mut self, value: u8) {
        self.0.push(value);
    }

    pub fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    pub fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    pub fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    pub fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    pub fn i64(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
}

/// Reads what `Writer` wrote; running out of bytes is a typed error.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], AiError> {
        let end = self.at.checked_add(N).ok_or(AiError::CorruptState)?;
        let bytes = self.data.get(self.at..end).ok_or(AiError::CorruptState)?;
        self.at = end;
        Ok(bytes.try_into().expect("slice has the requested length"))
    }

    pub fn u8(&mut self) -> Result<u8, AiError> {
        Ok(self.take::<1>()?[0])
    }

    pub fn bool(&mut self) -> Result<bool, AiError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(AiError::CorruptState),
        }
    }

    pub fn u32(&mut self) -> Result<u32, AiError> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    pub fn i32(&mut self) -> Result<i32, AiError> {
        Ok(i32::from_le_bytes(self.take()?))
    }

    pub fn u64(&mut self) -> Result<u64, AiError> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    pub fn i64(&mut self) -> Result<i64, AiError> {
        Ok(i64::from_le_bytes(self.take()?))
    }

    /// A collection length, bounded by the bytes that remain so a corrupt length cannot
    /// ask for a huge allocation.
    pub fn len(&mut self) -> Result<usize, AiError> {
        let len = self.u32()? as usize;
        if len > self.data.len() - self.at {
            return Err(AiError::CorruptState);
        }
        Ok(len)
    }

    /// Whether every byte has been read.
    pub fn is_done(&self) -> bool {
        self.at == self.data.len()
    }
}
