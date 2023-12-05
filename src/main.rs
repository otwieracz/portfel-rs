use crate::fx::Currency;

mod error;
mod fx;
mod portfolio;

fn main() {
    let portfolio = portfolio::Portfolio::from_file("portfolio.yaml").unwrap();
    println!("Portfolio: {}", portfolio);
    println!(
        "Balanced portfolio after:\n {}",
        portfolio.balanced(portfolio::Amount::new(Currency::USD, 6000.0))
    );
}
