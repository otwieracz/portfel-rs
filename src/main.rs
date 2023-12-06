use crate::fx::Currency;

mod error;
mod fx;
mod portfolio;

fn main() {
    let portfolio = portfolio::Portfolio::from_file("portfolio.yaml").unwrap();
    println!("Portfolio: {}", portfolio);
    // println!(
    //     "Balanced portfolio after:\n {:?}",
    //     portfolio.balance(portfolio::Amount::new(Currency::USD, 6000.0))
    // );
    let change_request = portfolio.balance(portfolio::Amount::new(Currency::USD, 6000.0));
    println!(
        "Balanced portfolio per group:\n {}",
        serde_yaml::to_string(&change_request).unwrap()
    );
}
