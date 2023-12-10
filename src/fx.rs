use std::collections::HashMap;

use serde::Deserialize;

use crate::{amount::Currency, error};

#[derive(Deserialize)]
struct SingleRateResponse {
    mid: f64,
}

#[derive(Deserialize)]
struct ExchangeRateResponse {
    rates: Vec<SingleRateResponse>,
}

#[derive(Debug, Clone)]
pub struct Rates {
    pub rates: HashMap<Currency, f64>,
}

impl Default for Rates {
    fn default() -> Self {
        Rates {
            rates: HashMap::new(),
        }
    }
}

async fn get_rate(currency: Currency) -> Result<f64, error::FxError> {
    let url = format!(
        "http://api.nbp.pl/api/exchangerates/rates/a/{}",
        currency.to_str()
    );
    Ok(reqwest::get(&url)
        .await
        .map_err(error::FxError::HttpError)?
        .json::<ExchangeRateResponse>()
        .await
        .map_err(error::FxError::JsonError)?
        .rates
        .first()
        .ok_or(error::FxError::GenericParserError)?
        .mid)
}

impl Rates {
    pub async fn load() -> Rates {
        let mut rates = HashMap::new();
        for currency in vec![Currency::USD, Currency::EUR, Currency::GBP, Currency::CHF] {
            rates.insert(currency, get_rate(currency).await.unwrap());
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
