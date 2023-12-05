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
    GenericError
}