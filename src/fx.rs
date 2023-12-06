use std::{collections::HashMap};

use serde::{Deserialize, Serialize};

use crate::error;

#[derive(Deserialize)]
struct SingleRateResponse {
    mid: f64,
}

#[derive(Deserialize)]
struct ExchangeRateResponse {
    rates: Vec<SingleRateResponse>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Currency {
    USD,
    EUR,
    GBP,
    CHF,
    PLN,
    NATIVE
}

impl Currency {
    fn from_str(s: &str) -> Option<Currency> {
        match s {
            "USD" => Some(Currency::USD),
            "EUR" => Some(Currency::EUR),
            "GBP" => Some(Currency::GBP),
            "CHF" => Some(Currency::CHF),
            "PLN" => Some(Currency::PLN),
            _ => None,
        }
    }

    fn to_str(&self) -> &str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::CHF => "CHF",
            Currency::PLN => "PLN",
            Currency::NATIVE => "PLN",
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Debug, Clone)]
pub struct Rates {
    pub rates: HashMap<Currency, f64>,
}

impl Default for Rates {
    fn default() -> Self {
        Self::new()
    }
}

fn get_rate(currency: Currency) -> Result<f64, error::FxError> {
    let url = format!(
        "http://api.nbp.pl/api/exchangerates/rates/a/{}",
        currency.to_str()
    );
    Ok(reqwest::blocking::get(&url)
        .map_err(error::FxError::HttpError)?
        .json::<ExchangeRateResponse>()
        .map_err(error::FxError::JsonError)?
        .rates
        .first()
        .ok_or(error::FxError::GenericParserError)?
        .mid)
}

impl Rates {
    pub fn new() -> Rates {
        let mut rates = HashMap::new();
        for currency in vec![Currency::USD, Currency::EUR, Currency::GBP, Currency::CHF] {
            rates.insert(currency, get_rate(currency).unwrap());
        }
        rates.insert(Currency::PLN, 1.0);
        rates.insert(Currency::NATIVE, 1.0);
        Rates { rates }
    }

    pub fn convert(&self, from: Currency, to: Currency, amount: f64) -> f64 {
        amount * self.rates.get(&from).unwrap() / self.rates.get(&to).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn compare_floats(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn test_rates() {
        let rates = Rates {
            rates: vec![
                (Currency::USD, 4.02),
                (Currency::EUR, 4.34),
                (Currency::GBP, 1.3),
                (Currency::CHF, 1.4),
                (Currency::PLN, 1.0),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(
            compare_floats(rates.convert(Currency::USD, Currency::USD, 100.0), 100.0),
            true
        );
        assert_eq!(
            compare_floats(rates.convert(Currency::USD, Currency::PLN, 100.0), 402.0),
            true
        );
        assert_eq!(
            compare_floats(rates.convert(Currency::EUR, Currency::PLN, 100.0), 434.0),
            true
        );
        assert_eq!(
            compare_floats(rates.convert(Currency::EUR, Currency::USD, 100.0), 107.96),
            true
        );
    }
}
