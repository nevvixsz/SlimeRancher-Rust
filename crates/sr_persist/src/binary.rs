//! Port of `PersistedDataSet.cs` and .NET `BinaryReader`/`BinaryWriter` primitives.

use std::collections::HashMap;
use std::hash::Hash;
use std::io::{self, Read, Seek, SeekFrom, Write};

use thiserror::Error;

pub const SECTION_SEPARATOR: i16 = 4096;
pub const ELEMENT_SEPARATOR: i16 = 8192;
pub const DATA_PAYLOAD_BEGIN: i16 = 12288;
pub const DATA_PAYLOAD_END: i16 = 16384;

#[derive(Debug, Error)]
pub enum PersistError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid UTF-8 string in save stream: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("invalid 7-bit encoded string length")]
    Invalid7BitInt,

    #[error("invalid dataset header: expected {expected_id} v{expected_ver}, got {got_id} v{got_ver}")]
    InvalidHeader {
        expected_id: String,
        expected_ver: u32,
        got_id: String,
        got_ver: u32,
    },

    #[error("unexpected marker {got} (expected {expected})")]
    UnexpectedMarker { expected: i16, got: i16 },

    #[error("invalid enum value {value} for {enum_name}")]
    InvalidEnum { enum_name: &'static str, value: i32 },

    #[error("invalid list/dictionary count: {0}")]
    InvalidCount(i32),

    #[error("save file not found: {0}")]
    NotFound(String),
}

pub type PersistResult<T> = Result<T, PersistError>;

/// Binary reader matching .NET `System.IO.BinaryReader` (Little-Endian, 7-bit UTF-8 strings).
pub struct BinReader<R> {
    inner: R,
}

impl<R: Read> BinReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn read_bool(&mut self) -> PersistResult<bool> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        Ok(buf[0] != 0)
    }

    pub fn read_i16(&mut self) -> PersistResult<i16> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    pub fn read_i32(&mut self) -> PersistResult<i32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    pub fn read_u32(&mut self) -> PersistResult<u32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn read_i64(&mut self) -> PersistResult<i64> {
        let mut buf = [0u8; 8];
        self.inner.read_exact(&mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }

    pub fn read_u64(&mut self) -> PersistResult<u64> {
        let mut buf = [0u8; 8];
        self.inner.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    pub fn read_f32(&mut self) -> PersistResult<f32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    pub fn read_f64(&mut self) -> PersistResult<f64> {
        let mut buf = [0u8; 8];
        self.inner.read_exact(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }

    /// Reads a .NET 7-bit encoded integer (`BinaryReader.Read7BitEncodedInt`).
    pub fn read_7bit_int(&mut self) -> PersistResult<i32> {
        let mut count: i32 = 0;
        let mut shift: u32 = 0;
        loop {
            if shift == 35 {
                return Err(PersistError::Invalid7BitInt);
            }
            let mut b = [0u8; 1];
            self.inner.read_exact(&mut b)?;
            let byte = b[0];
            count |= i32::from(byte & 0x7F) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                return Ok(count);
            }
        }
    }

    /// Reads a .NET `BinaryReader.ReadString()` (7-bit byte-length prefix + UTF-8).
    pub fn read_string(&mut self) -> PersistResult<String> {
        let len = self.read_7bit_int()?;
        if len < 0 {
            return Err(PersistError::InvalidCount(len));
        }
        let mut buf = vec![0u8; len as usize];
        self.inner.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf)?)
    }

    pub fn read_nullable_f64(&mut self) -> PersistResult<Option<f64>> {
        if self.read_bool()? {
            Ok(Some(self.read_f64()?))
        } else {
            Ok(None)
        }
    }

    pub fn read_nullable_f32(&mut self) -> PersistResult<Option<f32>> {
        if self.read_bool()? {
            Ok(Some(self.read_f32()?))
        } else {
            Ok(None)
        }
    }

    pub fn read_marker(&mut self, expected: i16) -> PersistResult<()> {
        let got = self.read_i16()?;
        if got != expected {
            return Err(PersistError::UnexpectedMarker { expected, got });
        }
        Ok(())
    }

    pub fn read_header(&mut self, expected_id: &str, expected_ver: u32) -> PersistResult<()> {
        let got_id = self.read_string()?;
        let got_ver = self.read_u32()?;
        if got_id != expected_id || got_ver != expected_ver {
            return Err(PersistError::InvalidHeader {
                expected_id: expected_id.to_string(),
                expected_ver,
                got_id,
                got_ver,
            });
        }
        Ok(())
    }

    pub fn read_list<T, F>(&mut self, mut read_fn: F) -> PersistResult<Vec<T>>
    where
        F: FnMut(&mut Self) -> PersistResult<T>,
    {
        let count = self.read_i32()?;
        if count < 0 {
            return Err(PersistError::InvalidCount(count));
        }
        let mut out = Vec::with_capacity(count as usize);
        for _ in 0..count {
            out.push(read_fn(self)?);
        }
        Ok(out)
    }

    /// Reads a dictionary serialized by `PersistedDataSet.WriteDictionary` (with `ELEMENT_SEPARATOR`).
    pub fn read_dict<K, V, FK, FV>(
        &mut self,
        mut key_fn: FK,
        mut val_fn: FV,
    ) -> PersistResult<HashMap<K, V>>
    where
        K: Eq + Hash,
        FK: FnMut(&mut Self) -> PersistResult<K>,
        FV: FnMut(&mut Self) -> PersistResult<V>,
    {
        let count = self.read_i32()?;
        if count < 0 {
            return Err(PersistError::InvalidCount(count));
        }
        let mut map = HashMap::with_capacity(count as usize);
        for _ in 0..count {
            let k = key_fn(self)?;
            let v = val_fn(self)?;
            self.read_marker(ELEMENT_SEPARATOR)?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn read_persistable_opt<T, F>(&mut self, read_fn: F) -> PersistResult<Option<T>>
    where
        F: FnOnce(&mut Self) -> PersistResult<T>,
    {
        if self.read_bool()? {
            Ok(Some(read_fn(self)?))
        } else {
            Ok(None)
        }
    }
}

impl<R: Read + Seek> BinReader<R> {
    pub fn stream_pos(&mut self) -> PersistResult<u64> {
        Ok(self.inner.stream_position()?)
    }

    pub fn seek_to(&mut self, pos: u64) -> PersistResult<()> {
        self.inner.seek(SeekFrom::Start(pos))?;
        Ok(())
    }
}

/// Binary writer matching .NET `System.IO.BinaryWriter` (Little-Endian, 7-bit UTF-8 strings).
pub struct BinWriter<W> {
    inner: W,
}

impl<W: Write> BinWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }

    pub fn write_bool(&mut self, v: bool) -> PersistResult<()> {
        self.inner.write_all(&[u8::from(v)])?;
        Ok(())
    }

    pub fn write_i16(&mut self, v: i16) -> PersistResult<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn write_i32(&mut self, v: i32) -> PersistResult<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn write_u32(&mut self, v: u32) -> PersistResult<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn write_i64(&mut self, v: i64) -> PersistResult<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn write_u64(&mut self, v: u64) -> PersistResult<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn write_f32(&mut self, v: f32) -> PersistResult<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn write_f64(&mut self, v: f64) -> PersistResult<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    /// Writes a .NET 7-bit encoded integer (`BinaryWriter.Write7BitEncodedInt`).
    pub fn write_7bit_int(&mut self, value: i32) -> PersistResult<()> {
        let mut v = value as u32;
        while v >= 0x80 {
            self.inner.write_all(&[((v & 0x7F) | 0x80) as u8])?;
            v >>= 7;
        }
        self.inner.write_all(&[v as u8])?;
        Ok(())
    }

    /// Writes a .NET `BinaryWriter.Write(string)` (7-bit byte-length prefix + UTF-8).
    pub fn write_string(&mut self, s: &str) -> PersistResult<()> {
        let bytes = s.as_bytes();
        self.write_7bit_int(bytes.len() as i32)?;
        self.inner.write_all(bytes)?;
        Ok(())
    }

    pub fn write_nullable_f64(&mut self, v: Option<f64>) -> PersistResult<()> {
        match v {
            Some(x) => {
                self.write_bool(true)?;
                self.write_f64(x)?;
            }
            None => self.write_bool(false)?,
        }
        Ok(())
    }

    pub fn write_nullable_f32(&mut self, v: Option<f32>) -> PersistResult<()> {
        match v {
            Some(x) => {
                self.write_bool(true)?;
                self.write_f32(x)?;
            }
            None => self.write_bool(false)?,
        }
        Ok(())
    }

    pub fn write_header(&mut self, identifier: &str, version: u32) -> PersistResult<()> {
        self.write_string(identifier)?;
        self.write_u32(version)?;
        Ok(())
    }

    pub fn write_list<T, F>(&mut self, items: &[T], mut write_fn: F) -> PersistResult<()>
    where
        F: FnMut(&mut Self, &T) -> PersistResult<()>,
    {
        self.write_i32(items.len() as i32)?;
        for item in items {
            write_fn(self, item)?;
        }
        Ok(())
    }

    pub fn write_dict<K, V, FK, FV>(
        &mut self,
        dict: &HashMap<K, V>,
        mut key_fn: FK,
        mut val_fn: FV,
    ) -> PersistResult<()>
    where
        FK: FnMut(&mut Self, &K) -> PersistResult<()>,
        FV: FnMut(&mut Self, &V) -> PersistResult<()>,
    {
        self.write_i32(dict.len() as i32)?;
        for (k, v) in dict {
            key_fn(self, k)?;
            val_fn(self, v)?;
            self.write_i16(ELEMENT_SEPARATOR)?;
        }
        Ok(())
    }

    pub fn write_persistable_opt<T, F>(
        &mut self,
        opt: Option<&T>,
        write_fn: F,
    ) -> PersistResult<()>
    where
        F: FnOnce(&mut Self, &T) -> PersistResult<()>,
    {
        match opt {
            Some(val) => {
                self.write_bool(true)?;
                write_fn(self, val)?;
            }
            None => self.write_bool(false)?,
        }
        Ok(())
    }
}

/// Trait implemented by every versioned Slime Rancher binary dataset (`Identifier` + `Version`).
pub trait PersistedDataset: Sized {
    const IDENTIFIER: &'static str;
    const VERSION: u32;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self>;
    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()>;

    fn load<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        reader.read_header(Self::IDENTIFIER, Self::VERSION)?;
        reader.read_marker(DATA_PAYLOAD_BEGIN)?;
        let val = Self::load_data(reader)?;
        reader.read_marker(DATA_PAYLOAD_END)?;
        Ok(val)
    }

    fn write<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_header(Self::IDENTIFIER, Self::VERSION)?;
        writer.write_i16(DATA_PAYLOAD_BEGIN)?;
        self.write_data(writer)?;
        writer.write_i16(DATA_PAYLOAD_END)?;
        Ok(())
    }
}
