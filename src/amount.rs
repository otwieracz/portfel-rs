use serde::{Deserialize, Serialize};
use strum::Display;
use strum_macros::EnumString;

use crate::fx::Rates;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Deserialize, Serialize, EnumString, Display)]
pub enum Currency {
    USD,
    EUR,
    GBP,
    JPY,
    CHF,
    PLN,
    NATIVE,
    #[serde(untagged)]
    UNKNOWN,
}

impl Currency {
    pub fn native() -> Currency {
        Currency::NATIVE
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

    pub fn convert(&self, currency: Currency, rates: &Rates) -> Amount {
        Amount {
            currency: currency,
            value: rates.convert(self.currency, currency, self.value),
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

impl std::ops::Add for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        assert!(
            self.currency == rhs.currency,
            "Cannot add amounts with different currencies: {} != {}",
            self.currency,
            rhs.currency
        );
        Self {
            currency: self.currency,
            value: self.value + rhs.value,
        }
    }
}
