use serde::{Deserialize, Serialize};

use crate::fx::Currency;

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
