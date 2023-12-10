use thiserror::Error;

#[derive(Debug)]
pub enum FxError {
    HttpError(reqwest::Error),
    JsonError(reqwest::Error),
    GenericParserError,
}

#[derive(Error, Debug)]
pub enum PortfolioReadError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_yaml::Error),
    #[error("Amount missing")]
    AmountMissing,
    #[error("Duplicate symbol: {0}")]
    DuplicateSymbolError(String),
    #[error("XTB error: {0}")]
    XtbError(#[from] XtbError),
    #[error("Crypt error: {0}")]
    CryptError(#[from] CryptError),
}

#[derive(Error, Debug)]
pub enum PortfolioWriteError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_yaml::Error),
    #[error("XTB error: {0}")]
    XtbError(#[from] XtbError),
    #[error("Crypt error: {0}")]
    CryptError(#[from] CryptError),
}

#[derive(Error, Debug)]
pub enum XtbError {
    #[error("Authentication error")]
    AuthenticationError,
    #[error("Password missing")]
    PasswordMissing,
    #[error("Not connected")]
    NotConnected,
    #[error("Native TLS error: {0}")]
    NativeTlsError(#[from] tokio_native_tls::native_tls::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Crypt error: {0}")]
    CryptError(#[from] CryptError),
    #[error("Unknown error")]
    UnknownError,
}
#[derive(Error, Debug)]
pub enum CryptError {
    #[error("Base64 error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Cipher error: {0}")]
    CipherError(#[from] openssl::error::ErrorStack),
}
