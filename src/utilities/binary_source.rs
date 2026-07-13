use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub(crate) trait BinarySource {
    fn read_bytes(&mut self, offset: u64, count: usize) -> io::Result<Vec<u8>>;
}

pub(crate) struct IbdFile {
    file: File,
    byte_count: u64,
}

impl IbdFile {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let byte_count = std::fs::metadata(path)?.len();
        Ok(Self {
            file: File::open(path)?,
            byte_count,
        })
    }

    pub(crate) fn byte_count(&self) -> u64 {
        self.byte_count
    }
}

impl BinarySource for IbdFile {
    fn read_bytes(&mut self, offset: u64, count: usize) -> io::Result<Vec<u8>> {
        let end = offset
            .checked_add(count as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "ibd byte range overflow"))?;
        if end > self.byte_count {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "ibd file is too small or does not match imzML: need bytes {}..{}, file has {} bytes",
                    offset, end, self.byte_count
                ),
            ));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0u8; count];
        self.file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}
