use std::collections::HashMap;

use crate::{error, fx, portfolio};
use serde::{Deserialize, Serialize};

use crate::fx::Currency;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Amount {
    currency: Currency,
    value: f64,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InvestmentGroup {
    id: String,
    currency: Currency,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Position {
    name: String,
    group: String,
    ticker: String,
    amount: Amount,
    target: f64,
}

impl std::ops::Sub for Position {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        assert!(
            self.amount.currency == rhs.amount.currency,
            "Cannot subtract positions with different currencies: {} != {}",
            self.amount.currency,
            rhs.amount.currency
        );
        Self {
            name: self.name,
            group: self.group,
            ticker: self.ticker,
            amount: self.amount - rhs.amount,
            target: self.target,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Portfolio {
    /* Internal */
    #[serde(skip)]
    rates: fx::Rates,
    /* Saved fields */
    positions: Vec<Position>,
}

impl std::fmt::Display for Portfolio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Total value: {:.2} {:?}",
            self.total_value(Currency::NATIVE).value,
            Currency::NATIVE
        )?;
        writeln!(f, "Positions:")?;
        write!(
            f,
            "{}",
            serde_yaml::to_string(&self.positions).map_err(|_| std::fmt::Error)?
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct PositionChange {
    position: Position,
    amount: Amount,
}

#[derive(Debug)]
pub struct ChangeRequest {
    changes: Vec<PositionChange>,
}

// Used only as a display helper
#[derive(Debug, Serialize)]
struct ChangeRequestSerialize {
    changes: Vec<PositionChange>,
    change_per_group: HashMap<String, Amount>,
}

impl Serialize for ChangeRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        ChangeRequestSerialize {
            changes: self.changes.clone(),
            change_per_group: self.change_per_group(),
        }
        .serialize(serializer)
    }
}

// impl std::fmt::Display for ChangeRequest {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let change_request_display = ChangeRequestSerialize {
//             changes: self.changes.clone(),
//             change_per_group: self.change_per_group(),
//         };
//         writeln!(
//             f,
//             "{}",
//             serde_yaml::to_string(&change_request_display).map_err(|_| std::fmt::Error)?
//         )?;
//         Ok(())
//     }
// }

impl ChangeRequest {
    pub fn change_per_group(&self) -> HashMap<String, Amount> {
        let mut change_per_group = HashMap::new();
        for change in &self.changes {
            let group = change.position.group.clone();
            let amount = change.amount.clone();
            let entry = change_per_group
                .entry(group)
                .or_insert(Amount::new(amount.currency, 0.0));
            entry.value += amount.value;
        }
        change_per_group
    }
}

impl Portfolio {
    pub fn new() -> Portfolio {
        Portfolio {
            rates: fx::Rates::new(),
            positions: Vec::new(),
        }
    }

    pub fn from_file(filename: &str) -> Result<Portfolio, error::PortfolioReadError> {
        let file = std::fs::File::open(filename).map_err(error::PortfolioReadError::IoError)?;
        let portfolio =
            serde_yaml::from_reader(file).map_err(error::PortfolioReadError::JsonError)?;
        Ok(portfolio)
    }

    fn total_value(&self, currency: Currency) -> Amount {
        let mut amount = Amount {
            currency: currency,
            value: 0.0,
        };

        for position in &self.positions {
            amount.value +=
                self.rates
                    .convert(position.amount.currency, currency, position.amount.value);
        }
        amount
    }

    pub fn balance(&self, investment: Amount) -> ChangeRequest {
        let total_value = self.total_value(investment.currency);

        let position_changes = self
            .positions
            .clone()
            .into_iter()
            .map(|position| {
                /* Values in investment currency */
                let old_value_in_currency = self.rates.convert(
                    position.amount.currency,
                    investment.currency,
                    position.amount.value,
                );
                let new_value_in_currency = position.target
                    * (investment.value + total_value.value)
                    - old_value_in_currency;

                let new_value = self.rates.convert(
                    investment.currency,
                    position.amount.currency,
                    new_value_in_currency,
                );

                PositionChange {
                    position: position.clone(),
                    amount: Amount {
                        currency: position.amount.currency,
                        value: new_value - position.amount.value,
                    },
                }
            })
            .collect();

        ChangeRequest {
            changes: position_changes,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_total_value() {
        let rates = fx::Rates {
            rates: vec![
                (Currency::USD, 1.0),
                (Currency::EUR, 1.2),
                (Currency::GBP, 1.3),
                (Currency::CHF, 1.4),
                (Currency::PLN, 1.0),
            ]
            .into_iter()
            .collect(),
        };

        let mut portfolio = Portfolio {
            rates: rates,
            positions: Vec::new(),
        };

        portfolio.positions.push(Position {
            name: "Test".to_string(),
            ticker: "TEST".to_string(),
            group: "TEST".to_string(),
            amount: Amount {
                currency: Currency::USD,
                value: 100.0,
            },
            target: 0.5,
        });
        portfolio.positions.push(Position {
            name: "Test".to_string(),
            ticker: "TEST".to_string(),
            group: "TEST".to_string(),
            amount: Amount {
                currency: Currency::EUR,
                value: 100.0,
            },
            target: 0.5,
        });
        assert_eq!(
            portfolio.total_value(Currency::USD),
            Amount {
                currency: Currency::USD,
                value: 100.0 * 1.0 + 100.0 * 1.2
            }
        );
    }

    #[test]
    fn test_balance_empty() {
        let rates = fx::Rates {
            rates: vec![
                (Currency::USD, 1.0),
                (Currency::EUR, 1.2),
                (Currency::GBP, 1.3),
                (Currency::CHF, 1.4),
                (Currency::PLN, 1.0),
            ]
            .into_iter()
            .collect(),
        };

        let portfolio = Portfolio {
            rates: rates,
            positions: vec![
                Position {
                    name: "Test 1".to_string(),
                    ticker: "TEST1".to_string(),
                    group: "TEST".to_string(),
                    amount: Amount {
                        currency: Currency::USD,
                        value: 0.0,
                    },
                    target: 0.3,
                },
                Position {
                    name: "Test 2".to_string(),
                    ticker: "TEST2".to_string(),
                    group: "TEST".to_string(),
                    amount: Amount {
                        currency: Currency::EUR,
                        value: 0.0,
                    },
                    target: 0.7,
                },
            ],
        };
        let investment = Amount {
            currency: Currency::USD,
            value: 1000.0,
        };

        let balanced = portfolio.balance(investment);
        assert_eq!(
            balanced.changes,
            vec![
                PositionChange {
                    position: Position {
                        name: "Test 1".to_string(),
                        ticker: "TEST1".to_string(),
                        group: "TEST".to_string(),
                        amount: Amount {
                            currency: Currency::USD,
                            value: 0.0,
                        },
                        target: 0.3,
                    },
                    amount: Amount {
                        currency: Currency::USD,
                        value: 300.0,
                    },
                },
                PositionChange {
                    position: Position {
                        name: "Test 2".to_string(),
                        ticker: "TEST2".to_string(),
                        group: "TEST".to_string(),
                        amount: Amount {
                            currency: Currency::EUR,
                            value: 0.0,
                        },
                        target: 0.7,
                    },
                    amount: Amount {
                        currency: Currency::EUR,
                        value: 583.33,
                    },
                },
            ]
        );
    }
}
