use thiserror::Error;

#[derive(Debug)]
pub enum FxError {
    HttpError(reqwest::Error),
    JsonError(reqwest::Error),
    GenericParserError,
}

#[derive(Debug)]
pub enum PortfolioReadError {
    IoError(std::io::Error),
    JsonError(serde_yaml::Error),
}

#[derive(Debug)]
pub enum PortfolioBalanceError {
    GenericError,
}

#[derive(Error, Debug)]
pub enum XtbError {
    #[error("Authentication error")]
    AuthenticationError,
    #[error("Not connected")]
    NotConnected,
    #[error("Native TLS error: {0}")]
    NativeTlsError(#[from] tokio_native_tls::native_tls::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unknown currency")]
    UnknownCurrency,
    #[error("Unknown error")]
    UnknownError,
}
