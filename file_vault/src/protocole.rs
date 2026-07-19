use std::{io::{Read}};

use crate::files_vault_errors::FileVaultError;

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

    pub fn from_bytes<R: Read>(buffer: &mut R) -> Result<Self, FileVaultError> {
        let mut get_name_size = [0u8; 4];
        buffer.read_exact(&mut get_name_size)
            .map_err(|_| FileVaultError::PacketCorrupted)?;
        let name_size = u32::from_be_bytes(get_name_size);

        let mut get_name = vec![0u8; name_size as usize];
        buffer.read_exact(get_name.as_mut_slice())
            .map_err(|_| FileVaultError::PacketCorrupted)?;
        let name = String::from_utf8(get_name)
            .map_err(|_| FileVaultError::IncorrectDataType)?;

        let mut get_data_size = [0u8; 8];
        buffer.read_exact(&mut get_data_size)
            .map_err(|_| FileVaultError::PacketCorrupted)?;
        let data_size = u64::from_be_bytes(get_data_size);

        Ok(Packet {
            name_size: name_size as u32,
            name,
            data_size,
        })
    }

    pub fn to_byte(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        
        bytes.extend_from_slice(&self.name_size.to_be_bytes());
        bytes.extend_from_slice(&self.name.as_bytes());
        bytes.extend_from_slice(&self.data_size.to_be_bytes());

        bytes
    }

    pub fn data_size(&self) -> u64 {
        self.data_size
    }

    pub fn get_name(&self) -> String {
        return self.name.clone();
    }
}