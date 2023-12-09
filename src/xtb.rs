use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio_native_tls::TlsConnector;
use tokio_native_tls::{native_tls, TlsStream};

use crate::amount::Amount;
use crate::{error, fx};

use self::command::get_trades::Trade;

pub mod command {
    use std::collections::HashMap;

    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct Command {
        pub command: String,
        pub arguments: HashMap<String, serde_json::Value>,
    }

    pub mod login {
        use super::Command;
        use serde::Deserialize;
        use std::collections::HashMap;

        pub fn login(account_id: String, password: String) -> Command {
            let mut arguments = HashMap::new();
            arguments.insert("userId".to_string(), account_id.into());
            arguments.insert("password".to_string(), password.into());
            Command {
                command: "login".to_string(),
                arguments,
            }
        }

        #[derive(Debug, Deserialize)]
        pub struct Response {
            pub status: bool,
            pub streamSessionId: Option<String>,
        }
    }

    pub mod get_trades {
        use serde::Deserialize;

        use super::Command;
        use std::collections::HashMap;

        pub fn get_trades(opened_only: bool) -> Command {
            let mut arguments = HashMap::new();
            arguments.insert("openedOnly".to_string(), opened_only.into());
            Command {
                command: "getTrades".to_string(),
                arguments,
            }
        }

        #[derive(Debug, Deserialize)]
        pub struct Response {
            pub status: bool,
            pub returnData: Option<Vec<Trade>>,
        }

        #[derive(Debug, Deserialize)]
        pub struct Trade {
            pub symbol: Option<String>,
            pub volume: f64,
        }
    }

    pub mod get_symbol {
        use serde::Deserialize;

        use crate::fx;

        use super::Command;
        use std::collections::HashMap;

        pub fn get_symbol(symbol: String) -> Command {
            let mut arguments = HashMap::new();
            arguments.insert("symbol".to_string(), symbol.into());
            Command {
                command: "getSymbol".to_string(),
                arguments,
            }
        }

        #[derive(Debug, Deserialize)]
        pub struct Response {
            pub status: bool,
            pub returnData: SymbolRecord,
        }

        #[derive(Debug, Deserialize)]
        pub struct SymbolRecord {
            pub bid: f64,
            pub symbol: String,
            pub currencyProfit: fx::Currency,
        }
    }
}

#[derive(Debug)]
pub struct PositionMarketValue {
    pub symbol: String,
    pub volume: f64,
    pub bid_price: Amount,
    pub market_value: Amount,
}

type Stream = BufReader<TlsStream<TcpStream>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct XtbConfig {
    host: String,
    port: u16,
    #[serde(skip)]
    stream: Option<Arc<Mutex<Stream>>>,
    #[serde(skip)]
    stream_session_id: Option<String>,
}

impl XtbConfig {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            stream: None,
            stream_session_id: None,
        }
    }

    /* Send arbitrary command that implements Serialize */
    async fn send_command<T: Serialize>(&self, command: T) -> Result<String, error::XtbError> {
        if let Some(stream) = self.stream.clone() {
            let mut stream = stream.lock().unwrap_or_else(PoisonError::into_inner);

            // Serialize the JSON command to a string
            let json_string = serde_json::to_string(&command).unwrap();

            // Send the JSON command to the server
            tokio::io::AsyncWriteExt::write_all(&mut *stream, json_string.as_bytes()).await?;

            // Read the response from the server until two newline characters are encountered
            let mut response = String::new();
            loop {
                let bytes_read =
                    tokio::io::AsyncBufReadExt::read_line(&mut *stream, &mut response).await?;
                if bytes_read == 0 || response.ends_with("\n\n") {
                    break;
                }
            }
            Ok(response)
        } else {
            return Err(error::XtbError::NotConnected);
        }
    }

    pub async fn connect(&mut self) -> Result<(), error::XtbError> {
        // Connect to the server
        let tcp_stream = TcpStream::connect((self.host.clone(), self.port)).await?;

        // Use a TlsConnector to establish an SSL/TLS connection
        let tls_connector = TlsConnector::from(native_tls::TlsConnector::new()?);
        let tls_stream = tls_connector.connect(&self.host, tcp_stream).await?;

        // Wrap the stream in a BufReader and BufWriter for efficient reading and writing
        let reader = BufReader::new(tls_stream);
        self.stream = Some(Arc::new(Mutex::new(reader)));

        Ok(())
    }

    pub async fn login(&mut self, account: XtbAccount) -> Result<(), error::XtbError> {
        let command = command::login::login(account.account_id, account.password);
        let response = self.send_command(command).await?;
        let response: command::login::Response = serde_json::from_str(&response)?;
        match response.status {
            false => Err(error::XtbError::AuthenticationError),
            true => {
                self.stream_session_id = response.streamSessionId;
                Ok(())
            }
        }
    }

    async fn get_trades(&self, opened_only: bool) -> Result<Vec<Trade>, error::XtbError> {
        let command = command::get_trades::get_trades(opened_only);
        let response = self.send_command(command).await?;
        let response: command::get_trades::Response = serde_json::from_str(&response)?;
        match response.status {
            false => Err(error::XtbError::UnknownError),
            true => {
                if let Some(trades) = response.returnData {
                    Ok(trades)
                } else {
                    Ok(vec![])
                }
            }
        }
    }

    async fn get_symbol(
        &self,
        symbol: String,
    ) -> Result<command::get_symbol::SymbolRecord, error::XtbError> {
        let command = command::get_symbol::get_symbol(symbol);
        let response = self.send_command(command).await?;
        let response: command::get_symbol::Response = serde_json::from_str(&response)?;
        match response.status {
            false => Err(error::XtbError::UnknownError),
            true => Ok(response.returnData),
        }
    }

    pub async fn get_position_market_values(
        &self,
    ) -> Result<Vec<PositionMarketValue>, error::XtbError> {
        let trades = self.get_trades(true).await?;

        let symbols: Vec<String> = trades
            .iter()
            .map(|trade| trade.symbol.clone().unwrap())
            .collect();

        /* use get_symbol */
        let mut symbol_records = vec![];
        for symbol in symbols {
            let tick_price = self.get_symbol(symbol.clone()).await?;
            symbol_records.push(tick_price);
        }

        let mut position_market_values = vec![];

        for trade in trades {
            let symbol_record = symbol_records
                .iter()
                .find(|tick_price| tick_price.symbol == trade.symbol.clone().unwrap())
                .unwrap();
            position_market_values.push(PositionMarketValue {
                symbol: trade.symbol.unwrap(),
                volume: trade.volume,
                bid_price: Amount::new(symbol_record.currencyProfit, symbol_record.bid),
                market_value: Amount::new(
                    symbol_record.currencyProfit,
                    trade.volume * symbol_record.bid,
                ),
            });
        }
        Ok(position_market_values)
    }
}

impl Default for XtbConfig {
    fn default() -> Self {
        Self {
            host: "xapi.xtb.com".to_string(),
            port: 5124,
            stream: None,
            stream_session_id: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XtbAccount {
    account_id: String,
    password: String,
}

impl XtbAccount {
    pub fn new(account_id: &str, password: &str) -> Self {
        Self {
            account_id: account_id.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, result};

    use super::*;

    #[tokio::test]
    async fn failed_login_attempt() {
        let account = XtbAccount::new("123456", "password");
        let mut xtb = XtbConfig::default();
        xtb.connect().await.unwrap();
        let result = xtb.login(account).await;
        assert_eq!(
            matches!(result, Err(error::XtbError::AuthenticationError)),
            true
        );
    }

    #[tokio::test]
    async fn successful_login_attempt() {
        let account_id = env::var("XTB_TEST_DEMO_ACCOUNT_ID").ok();
        let password = env::var("XTB_TEST_DEMO_PASSWORD").ok();

        /* Enable this test only if env-vars are set */
        if account_id.is_some() && password.is_some() {
            let account = XtbAccount::new(&account_id.unwrap(), &password.unwrap());
            let mut xtb = XtbConfig::default();
            xtb.connect().await.unwrap();
            let result = xtb.login(account).await;
            assert_eq!(matches!(result, Ok(())), true);
        }
    }

    #[tokio::test]
    async fn get_trades() {
        let account_id = env::var("XTB_TEST_DEMO_ACCOUNT_ID").ok();
        let password = env::var("XTB_TEST_DEMO_PASSWORD").ok();

        /* Enable this test only if env-vars are set */
        if account_id.is_some() && password.is_some() {
            let account = XtbAccount::new(&account_id.unwrap(), &password.unwrap());
            // let mut xtb = XtbConfig::new("xapi.xtb.com".to_string(), 5112);
            let mut xtb = XtbConfig::default();
            xtb.connect().await.unwrap();
            xtb.login(account).await.unwrap();
            let result = xtb.get_trades(true).await;
            assert_eq!(matches!(result, Ok(_)), true);

        }
    }

    #[tokio::test]
    async fn get_position_market_values() {
        let account_id = env::var("XTB_TEST_DEMO_ACCOUNT_ID").ok();
        let password = env::var("XTB_TEST_DEMO_PASSWORD").ok();

        /* Enable this test only if env-vars are set */
        if account_id.is_some() && password.is_some() {
            let account = XtbAccount::new(&account_id.unwrap(), &password.unwrap());
            // let mut xtb = XtbConfig::new("xapi.xtb.com".to_string(), 5112);
            let mut xtb = XtbConfig::default();
            xtb.connect().await.unwrap();
            xtb.login(account).await.unwrap();
        }
    }
}
