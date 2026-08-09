use std::{io::Write, net::TcpStream};

use sha2::{Digest, Sha256};

pub struct HashingWriter<'a> {
    stream: &'a mut TcpStream,
    hasher: Sha256,
}

impl<'a> HashingWriter<'a> {
    pub fn new(stream: &'a mut TcpStream) -> Self {
        Self {
            stream,
            hasher: Sha256::new(),
        }
    }

    pub fn finalize(self) -> [u8; 32] {
        let digest = self.hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&digest);
        hash
    }
}

impl Write for HashingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let bytes_written = self.stream.write(buf)?;
        self.hasher.update(&buf[..bytes_written]);
        Ok(bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}