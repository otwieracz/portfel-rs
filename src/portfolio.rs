use std::collections::HashMap;

use crate::{
    amount::Amount,
    amount::Currency,
    error,
    fx::Rates,
    xtb::{self, XtbAccount, XtbConfig},
};
use good_lp::{constraint, default_solver, Expression, Solution, SolverModel};
use serde::{Deserialize, Serialize};

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
    /// `amount` is safe to unwrap
    ///
    /// It is possible to be `None` only when reading from file in cases where external provider like XTB is used.
    /// In such case, the amount will be read from the external provider and set to `Some` value. If this process
    /// were to fail, `from_file` would return an error.
    ///
    /// Any subsequent usages of `amount` should expect it to be `Some` and panic otherwise.
    amount: Option<Amount>,
    target: f64,
}

impl std::ops::Sub for Position {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let self_amount = self.amount.unwrap();
        let rhs_amount = rhs.amount.unwrap();
        assert!(
            self_amount.currency == rhs_amount.currency,
            "Cannot subtract positions with different currencies: {} != {}",
            self_amount.currency,
            rhs_amount.currency
        );
        Self {
            name: self.name,
            group: self.group,
            ticker: self.ticker,
            amount: Some(self_amount - rhs_amount),
            target: self.target,
        }
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let position_amount = self.amount.clone().unwrap();
        write!(
            f,
            "[{:8.8}] {:37.36}: {:9.2} {}",
            self.ticker.to_string(),
            self.name.to_string(),
            position_amount.value,
            position_amount.currency,
        )?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Group {
    id: String,
    currency: Currency,
    xtb: Option<xtb::XtbAccount>,
}

impl Group {
    #[allow(dead_code)]
    pub fn new(id: String, currency: Currency) -> Group {
        Group {
            id: id,
            currency: currency,
            xtb: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    xtb: Option<xtb::XtbConfig>,
    #[serde(default = "Currency::native")]
    base_currency: Currency,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Portfolio {
    /* Internal */
    #[serde(skip)]
    rates: Rates,
    /* Saved fields */
    config: Config,
    groups: Vec<Group>,
    positions: Vec<Position>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            xtb: None,
            base_currency: Currency::native(),
        }
    }
}

impl std::fmt::Display for Portfolio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Total value: {:.2} {:?}",
            self.total_value(self.config.base_currency).value,
            self.config.base_currency
        )?;
        writeln!(f, "Positions:")?;
        for position in &self.positions {
            let position_amount = position.amount.clone().unwrap();
            let position_share =
                position_amount.value / self.total_value(position_amount.currency).value;

            writeln!(
                f,
                "- {} [{:4.2} ({:4.2})]",
                position, position_share, position.target
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct PositionChange {
    position: Position,
    amount: Amount,
}

impl PositionChange {
    fn new_value(&self) -> Amount {
        let position_amount = self.position.amount.clone().unwrap();
        Amount {
            currency: position_amount.currency,
            value: position_amount.value + self.amount.value,
        }
    }

    fn format(&self, rates: &Rates, total_portfolio_value: Amount) -> String {
        let position_share = self.new_value().div(&total_portfolio_value, &rates);
        // Use regular display method, but add share
        format!(
            "{} [{:4.2} ({:4.2})]",
            self, position_share, self.position.target
        )
    }
}
impl std::fmt::Display for PositionChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let position_amount = self.position.amount.clone().unwrap();
        write!(
            f,
            "[{:8.8}] {:37.36}: {:9.2} {} -[+ {:9.2} {}]> {:9.2} {}",
            self.position.ticker.to_string(),
            self.position.name.to_string(),
            position_amount.value,
            position_amount.currency,
            self.amount.value,
            self.amount.currency,
            position_amount.value + self.amount.value,
            position_amount.currency
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ChangeRequest {
    changes: Vec<PositionChange>,
}

impl ChangeRequest {
    pub fn format(&self, portfolio: &Portfolio) -> String {
        let mut result = String::new();
        let current_value = portfolio.total_value(portfolio.config.base_currency).value;
        let total_change = self
            .total_change(&portfolio.rates, portfolio.config.base_currency)
            .value;
        let new_value = current_value + total_change;

        result.push_str("Change requests:\n");
        for change in &self.changes {
            result.push_str(&format!(
                "{}\n",
                change.format(
                    &portfolio.rates,
                    portfolio.total_value(portfolio.config.base_currency).add(
                        &self.total_change(&portfolio.rates, portfolio.config.base_currency),
                        &portfolio.rates,
                    )
                )
            ));
        }
        result.push_str("\nChange per group:\n");
        for (group, amount) in self.change_per_group() {
            result.push_str(&format!(
                "- {:16.47}: + {:9.2} {}\n",
                group, amount.value, amount.currency
            ));
        }
        result.push_str(&format!(
            "\nTotal: {:9.2} + {:9.2} = {:9.2} {}\n",
            current_value, total_change, new_value, portfolio.config.base_currency,
        ));

        result
    }
}

impl ChangeRequest {
    pub fn change_per_group(&self) -> HashMap<String, Amount> {
        // TODO: Use actual group config for currency, not position currency
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

    pub fn total_change(&self, rates: &Rates, currency: Currency) -> Amount {
        let mut total_change = Amount::new(currency, 0.0);
        for change in &self.changes {
            total_change.value +=
                rates.convert(change.amount.currency, currency, change.amount.value);
        }
        total_change
    }
}

impl Portfolio {
    #[allow(dead_code)]
    pub async fn new() -> Portfolio {
        Portfolio {
            rates: Rates::load().await,
            config: Config::default(),
            groups: Vec::new(),
            positions: Vec::new(),
        }
    }

    /// Initialize portfolio with example data
    pub fn example(xtb_config: Option<XtbConfig>, xtb_account: Option<XtbAccount>) -> Portfolio {
        Portfolio {
            rates: Rates::default(),
            config: Config {
                xtb: xtb_config,
                base_currency: Currency::USD,
            },
            groups: vec![
                Group {
                    id: "xtb_usd".to_string(),
                    currency: Currency::USD,
                    xtb: xtb_account.clone(),
                },
                Group {
                    id: "cash_eur".to_string(),
                    currency: Currency::EUR,
                    xtb: None,
                },
            ],
            positions: vec![
                Position {
                    name: "S&P 500".to_string(),
                    ticker: "SPX500".to_string(),
                    group: "xtb_usd".to_string(),
                    amount: {
                        if xtb_account.is_some() {
                            None
                        } else {
                            Some(Amount {
                                currency: Currency::USD,
                                value: 100.0,
                            })
                        }
                    },
                    target: 0.5,
                },
                Position {
                    name: "Cash".to_string(),
                    ticker: "CASH".to_string(),
                    group: "cash_eur".to_string(),
                    amount: Some(Amount {
                        currency: Currency::EUR,
                        value: 100.0,
                    }),
                    target: 0.5,
                },
            ],
        }
    }

    pub async fn from_file(
        filename: &str,
        encryption_key: &str,
    ) -> Result<Portfolio, error::PortfolioReadError> {
        let file = std::fs::File::open(filename)?;
        let mut portfolio: Portfolio = serde_yaml::from_reader(file)?;

        /* Load rates */
        portfolio.rates = Rates::load().await;

        /* Read market values from xtb */
        let mut xtb_position_market_values: HashMap<String, Amount> = HashMap::new();
        if let Some(mut xtb) = portfolio.config.xtb.clone() {
            // Get market values for all positions from XTB for each group
            for group in &mut portfolio.groups {
                if let Some(xtb_account) = group.xtb.clone() {
                    xtb.connect().await?;
                    xtb.login(&xtb_account.decrypt(&encryption_key)?).await?;

                    let group_position_market_values: Result<Vec<_>, _> = xtb
                        .get_position_market_values()
                        .await?
                        .into_iter()
                        /* Filter only position from this group */
                        .filter(|xtb_position| {
                            portfolio.positions.iter().any(|position| {
                                position.ticker == xtb_position.symbol && position.group == group.id
                            })
                        })
                        .map(|x| {
                            // Currently same positions in different groups are not supported
                            // Check if position with this symbol already exists in global xtb_position_market_values
                            if xtb_position_market_values.contains_key(&x.symbol) {
                                return Err(error::PortfolioReadError::DuplicateSymbolError(
                                    x.symbol,
                                ));
                            } else {
                                return Ok((
                                    x.symbol,
                                    x.market_value.convert(group.currency, &portfolio.rates),
                                ));
                            }
                        })
                        .collect();
                    xtb.disconnect().await?;

                    for (symbol, market_value) in group_position_market_values? {
                        if xtb_position_market_values.contains_key(&symbol) {
                            log::info!(
                                "Duplicate symbol: {} in group: {}, adding",
                                symbol,
                                group.id.to_string()
                            );
                            xtb_position_market_values.insert(
                                symbol.clone(),
                                market_value + xtb_position_market_values[&symbol].clone(),
                            );
                        } else {
                            xtb_position_market_values.insert(symbol, market_value);
                        }
                    }
                }
            }
        }

        /* Set position market values from xtb if `amount` is none. */
        for position in &mut portfolio.positions {
            if position.amount.is_none() {
                let position_market_value = xtb_position_market_values
                    .get(&position.ticker)
                    .ok_or(error::PortfolioReadError::AmountMissing)?;
                position.amount = Some(position_market_value.clone());
            }
        }

        Ok(portfolio)
    }

    pub async fn to_file(&self, filename: &str) -> Result<String, error::PortfolioWriteError> {
        let mut file = std::fs::File::create(filename)?;
        serde_yaml::to_writer(&mut file, &self)?;

        Ok(filename.to_string())
    }

    fn total_value(&self, currency: Currency) -> Amount {
        let mut amount = Amount {
            currency,
            value: 0.0,
        };

        for position in &self.positions {
            amount.value += self.rates.convert(
                position.amount.clone().unwrap().currency,
                currency,
                position.amount.clone().unwrap().value,
            );
        }
        amount
    }

    /// Balance portfolio to given investment
    /// Returns a list of changes to be made to the portfolio
    pub fn balance(&self, investment: Amount) -> Result<ChangeRequest, error::PortfolioOpsError> {
        let mut problem_variables = good_lp::ProblemVariables::new();

        let current_portfolio_value = self.total_value(investment.currency).value;
        let mut per_position_investments = vec![];
        for _position in &self.positions {
            per_position_investments
                .push(problem_variables.add(good_lp::variable().min(0).max(investment.value)))
        }

        let total_investment: Expression = per_position_investments.iter().sum();
        let new_portfolio_value = investment.value + current_portfolio_value;

        let mut total_objective: Expression = 0.into();

        let objectives: Vec<_> = self
            .positions
            .clone()
            .into_iter()
            .zip(per_position_investments.clone().into_iter())
            .map(|(position, position_investment)| {
                // this position value in investment currency
                let position_value = self.rates.convert(
                    position.amount.clone().unwrap().currency,
                    investment.currency,
                    position.amount.clone().unwrap().value,
                );

                // Objective for specific position - minimize the imbalance
                let mut position_objective = ((position_value + position_investment)
                    / new_portfolio_value)
                    - position.target;

                // If current share < target share, negate the objective (approaching from below 0)
                let current_share = position_value / current_portfolio_value;
                if current_share < position.target {
                    position_objective = -position_objective;
                }

                // Add this position objective to total objective
                total_objective += position_objective.clone();
                position_objective
            })
            .collect();

        // Define the problem
        //
        // Minimise the sum of differences of each share from targe share
        // Constraint the total investment value to target investment value
        let mut problem = problem_variables
            .minimise(total_objective)
            .using(default_solver)
            .with(constraint!(total_investment == investment.value));

        // Constrint each position: share can't be negative
        for this in objectives {
            problem = problem.with(constraint!(this.clone() >= 0.0));
        }

        // Solve
        let solution = problem.solve()?;

        let changes: Vec<_> = self
            .positions
            .clone()
            .into_iter()
            .zip(per_position_investments.into_iter())
            .map(|(position, variable)| {
                let new_value = solution.value(variable.clone());
                let position_currency = position.clone().amount.unwrap().currency;
                let position_change = PositionChange {
                    position: position.clone(),
                    amount: Amount {
                        currency: position.amount.unwrap().currency,
                        value: self.rates.convert(
                            investment.currency,
                            position_currency,
                            new_value,
                        ),
                    },
                };
                position_change
            })
            .collect();

        Ok(ChangeRequest { changes })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn mock_rates() -> Rates {
        Rates {
            rates: vec![
                (Currency::USD, 1.0),
                (Currency::EUR, 1.2),
                (Currency::GBP, 1.3),
                (Currency::CHF, 1.4),
                (Currency::PLN, 1.0),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn test_total_value() {
        let rates = mock_rates();

        let mut portfolio = Portfolio {
            rates: rates,
            config: Config::default(),
            groups: vec![
                Group::new("TEST1".to_string(), Currency::USD),
                Group::new("TEST2".to_string(), Currency::EUR),
            ],
            positions: Vec::new(),
        };

        portfolio.positions.push(Position {
            name: "Test".to_string(),
            ticker: "TEST".to_string(),
            group: "TEST1".to_string(),
            amount: Some(Amount {
                currency: Currency::USD,
                value: 100.0,
            }),
            target: 0.5,
        });
        portfolio.positions.push(Position {
            name: "Test".to_string(),
            ticker: "TEST".to_string(),
            group: "TEST2".to_string(),
            amount: Some(Amount {
                currency: Currency::EUR,
                value: 100.0,
            }),
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
        let rates = mock_rates();

        let portfolio = Portfolio {
            config: Config::default(),
            groups: vec![
                Group::new("TEST1".to_string(), Currency::USD),
                Group::new("TEST2".to_string(), Currency::EUR),
            ],
            rates: rates,
            positions: vec![
                Position {
                    name: "Test 1".to_string(),
                    ticker: "TEST1".to_string(),
                    group: "TEST1".to_string(),
                    amount: Some(Amount {
                        currency: Currency::USD,
                        value: 0.0,
                    }),
                    target: 0.3,
                },
                Position {
                    name: "Test 2".to_string(),
                    ticker: "TEST2".to_string(),
                    group: "TEST2".to_string(),
                    amount: Some(Amount {
                        currency: Currency::EUR,
                        value: 0.0,
                    }),
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
            balanced.unwrap().changes,
            vec![
                PositionChange {
                    position: Position {
                        name: "Test 1".to_string(),
                        ticker: "TEST1".to_string(),
                        group: "TEST1".to_string(),
                        amount: Some(Amount {
                            currency: Currency::USD,
                            value: 0.0,
                        }),
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
                        group: "TEST2".to_string(),
                        amount: Some(Amount {
                            currency: Currency::EUR,
                            value: 0.0,
                        }),
                        target: 0.7,
                    },
                    amount: Amount {
                        currency: Currency::EUR,
                        value: 700.00 / 1.2,
                    },
                },
            ]
        );
    }

    #[test]
    fn test_balance_non_empty() {
        let rates = Rates {
            rates: vec![(Currency::USD, 1.0)].into_iter().collect(),
        };

        let portfolio = Portfolio {
            config: Config::default(),
            groups: vec![Group::new("TEST1".to_string(), Currency::USD)],
            rates: rates,
            positions: vec![
                Position {
                    name: "Test 1".to_string(),
                    ticker: "TEST1".to_string(),
                    group: "TEST1".to_string(),
                    amount: Some(Amount {
                        currency: Currency::USD,
                        value: 500.0,
                    }),
                    target: 0.5,
                },
                Position {
                    name: "Test 2".to_string(),
                    ticker: "TEST2".to_string(),
                    group: "TEST1".to_string(),
                    amount: Some(Amount {
                        currency: Currency::USD,
                        value: 500.0,
                    }),
                    target: 0.5,
                },
            ],
        };
        let investment = Amount {
            currency: Currency::USD,
            value: 1000.0,
        };

        let balanced = portfolio.balance(investment);
        assert_eq!(
            balanced.unwrap().changes,
            vec![
                PositionChange {
                    position: Position {
                        name: "Test 1".to_string(),
                        ticker: "TEST1".to_string(),
                        group: "TEST1".to_string(),
                        amount: Some(Amount {
                            currency: Currency::USD,
                            value: 500.0,
                        }),
                        target: 0.5,
                    },
                    amount: Amount {
                        currency: Currency::USD,
                        value: 500.0,
                    },
                },
                PositionChange {
                    position: Position {
                        name: "Test 2".to_string(),
                        ticker: "TEST2".to_string(),
                        group: "TEST1".to_string(),
                        amount: Some(Amount {
                            currency: Currency::USD,
                            value: 500.0,
                        }),
                        target: 0.5,
                    },
                    amount: Amount {
                        currency: Currency::USD,
                        value: 500.0,
                    },
                },
            ]
        );
    }

    #[test]
    fn test_balance_unbalancable() {
        let rates = Rates {
            rates: vec![(Currency::USD, 1.0)].into_iter().collect(),
        };

        let portfolio = Portfolio {
            config: Config::default(),
            groups: vec![Group::new("TEST1".to_string(), Currency::USD)],
            rates: rates,
            positions: vec![
                Position {
                    name: "Test 1".to_string(),
                    ticker: "TEST1".to_string(),
                    group: "TEST1".to_string(),
                    amount: Some(Amount {
                        currency: Currency::USD,
                        value: 100.0,
                    }),
                    target: 0.5,
                },
                Position {
                    name: "Test 2".to_string(),
                    ticker: "TEST2".to_string(),
                    group: "TEST1".to_string(),
                    amount: Some(Amount {
                        currency: Currency::USD,
                        value: 500.0,
                    }),
                    target: 0.5,
                },
            ],
        };
        let investment = Amount {
            currency: Currency::USD,
            value: 300.0,
        };

        let balanced = portfolio.balance(investment);
        assert_eq!(
            balanced.unwrap().changes,
            vec![
                PositionChange {
                    position: Position {
                        name: "Test 1".to_string(),
                        ticker: "TEST1".to_string(),
                        group: "TEST1".to_string(),
                        amount: Some(Amount {
                            currency: Currency::USD,
                            value: 100.0,
                        }),
                        target: 0.5,
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
                        group: "TEST1".to_string(),
                        amount: Some(Amount {
                            currency: Currency::USD,
                            value: 500.0,
                        }),
                        target: 0.5,
                    },
                    amount: Amount {
                        currency: Currency::USD,
                        value: 0.0,
                    },
                },
            ]
        );
    }
}
