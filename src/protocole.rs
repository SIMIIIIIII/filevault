use tokio::io::AsyncReadExt;

use crate::files_vault_errors::FileVaultError;
const MAX_NAME_SIZE: u32 = 4096;
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10 Go

#[derive(Debug, PartialEq, Clone)]
pub struct Packet {
    name_size: u32,
    name: String,
    data_size: u64,
}

impl Packet {
    pub fn from(name: String, data_size: u64) -> Self {
        let name_size: u32 = name.len() as u32;

        Packet {
            name_size,
            name,
            data_size,
        }
    }

    pub async fn from_bytes<R>(buffer: &mut R) -> Result<Self, FileVaultError>
    where
        R: AsyncReadExt + Unpin,
    {
        let mut get_name_size = [0u8; 4];
        buffer
            .read_exact(&mut get_name_size)
            .await
            .map_err(|_| FileVaultError::PacketCorrupted)?;
        let name_size = u32::from_be_bytes(get_name_size);

        if name_size > MAX_NAME_SIZE {
            return Err(FileVaultError::PacketCorrupted);
        }

        let mut get_name = vec![0u8; name_size as usize];
        buffer
            .read_exact(get_name.as_mut_slice())
            .await
            .map_err(|_| FileVaultError::PacketCorrupted)?;
        let name = String::from_utf8(get_name).map_err(|_| FileVaultError::IncorrectDataType)?;

        let mut get_data_size = [0u8; 8];
        buffer
            .read_exact(&mut get_data_size)
            .await
            .map_err(|_| FileVaultError::PacketCorrupted)?;
        let data_size = u64::from_be_bytes(get_data_size);

        if data_size > MAX_FILE_SIZE {
            return Err(FileVaultError::PacketCorrupted);
        }

        Ok(Packet {
            name_size,
            name,
            data_size,
        })
    }

    pub fn to_byte(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        bytes.extend_from_slice(&self.name_size.to_be_bytes());
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.extend_from_slice(&self.data_size.to_be_bytes());

        bytes
    }

    pub fn data_size(&self) -> u64 {
        self.data_size
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}
