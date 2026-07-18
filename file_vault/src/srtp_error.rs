use core::fmt;

#[derive(Debug, PartialEq)]
pub enum FileVaultError {
    PacketTooLong(String),
    InconsistantValue(String, String),
    MissingField(String),
    MissingPayload,
    PresentPayload,
    IncorrectLength(usize),
    IncorrectDataType,
    InvalidURL(String),
    SendError(String),
    ZeroPacketsReceived,
    TimoutError,
    SocketCreationError(String),
    TooManyInvalidPascket(u8),
    ServerError
}

impl fmt::Display for FileVaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InconsistantValue(value, field) => write!(f, "The value '{value}' is inconsistant for field '{field}'"),
            Self::MissingField(field) => write!(f, "The field '{field}' is missing"),
            Self::PacketTooLong(length) => write!(f, "Incorrect packet length: '{length}'"),
            Self::MissingPayload => write!(f, "The payload is missing for an DATA packet"),
            Self::PresentPayload => write!(f, "The payload should miss for an ACK packet"),
            Self::IncorrectLength(length) => write!(f, "Incorrect bytes length: '{length}'"),
            Self::IncorrectDataType => write!(f, "Incorrect packet type"),
            Self::InvalidURL(url) => write!(f, "URL is invalid: '{url}'"),
            Self::SendError(e) => write!(f, "Error while sending: {e}"),
            Self::ZeroPacketsReceived => write!(f, "Zero Packets received"),
            Self::TimoutError => write!(f, "Error when setting a timeout"),
            Self::SocketCreationError(e) => write!(f, "Error while creating socket: {e}"),
            Self::TooManyInvalidPascket(n) => write!(f, "Too many invalid packet: {n}"),
            Self::ServerError => write!(f, "Server Error"),
        }
    }
}