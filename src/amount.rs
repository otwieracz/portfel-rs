use serde::{Deserialize, Serialize};

use crate::fx::Rates;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Currency {
    USD,
    EUR,
    GBP,
    CHF,
    PLN,
    NATIVE,
    UNKNOWN,
}

impl Currency {
    pub fn from_str(s: &str) -> Option<Currency> {
        match s {
            "USD" => Some(Currency::USD),
            "EUR" => Some(Currency::EUR),
            "GBP" => Some(Currency::GBP),
            "CHF" => Some(Currency::CHF),
            "PLN" => Some(Currency::PLN),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::CHF => "CHF",
            Currency::PLN => "PLN",
            Currency::NATIVE => "PLN",
            Currency::UNKNOWN => panic!("Unknown currency!"),
        }
    }

    pub fn native() -> Currency {
        Currency::NATIVE
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Amount {
    pub currency: Currency,
    pub value: f64,
}

fn compare_floats(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

impl PartialEq for Amount {
    fn eq(&self, other: &Self) -> bool {
        self.currency == other.currency && compare_floats(self.value, other.value)
    }
}

impl Amount {
    pub fn new(currency: Currency, value: f64) -> Amount {
        Amount {
            currency: currency,
            value: value,
        }
    }

    pub fn div(&self, other: &Amount, rates: &Rates) -> f64 {
        if self.currency == other.currency {
            self.value / other.value
        } else {
            self.value / rates.convert(other.currency, self.currency, other.value)
        }
    }

    pub fn add(&self, other: &Amount, rates: &Rates) -> Amount {
        if self.currency == other.currency {
            Amount {
                currency: self.currency,
                value: self.value + other.value,
            }
        } else {
            Amount {
                currency: self.currency,
                value: self.value + rates.convert(other.currency, self.currency, other.value),
            }
        }
    }
}

impl std::ops::Sub for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        assert!(
            self.currency == rhs.currency,
            "Cannot subtract amounts with different currencies: {} != {}",
            self.currency,
            rhs.currency
        );
        Self {
            currency: self.currency,
            value: self.value - rhs.value,
        }
    }
}
