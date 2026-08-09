use core::fmt;

#[derive(Debug, PartialEq)]
pub enum FileVaultError {
    MissingPayload,
    IncorrectDataType,
    HashMismatch,
    ConnectionError(String),
    FileOpeningError(String),
    TcpSendingError(String),
    FileWritingError(String),
    PacketCorrupted
}

impl fmt::Display for FileVaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPayload => write!(f, "The payload is missing for an DATA packet"),
            Self::IncorrectDataType => write!(f, "Incorrect packet type"),
            Self::HashMismatch => write!(f, "SHA-256 hash mismatch"),
            Self::ConnectionError(e) => write!(f, "TcpStream connection Error: {e}"),
            Self::FileOpeningError(e) => write!(f, "Error while Opening file : {e}"),
            Self::FileWritingError(e) => write!(f, "Error while writing in file : {e}"),
            Self::TcpSendingError(e) => write!(f, "TcpStream sending error: {e}"),
            Self::PacketCorrupted => write!(f, "The packet is corrupted")
        }
    }
}