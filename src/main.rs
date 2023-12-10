use crate::{amount::Amount, amount::Currency};
use clap::{Parser, Subcommand};

mod amount;
mod error;
mod fx;
mod portfolio;
mod xtb;

#[derive(Subcommand)]
enum Commands {
    Show,
    Invest {
        #[arg(short, long)]
        amount: f64,
        #[arg(short, long)]
        currency: String,
    },
}

#[derive(Parser)]
struct Cli {
    #[clap(short, long, value_name = "YAML")]
    portfolio: String,
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let portfolio = portfolio::Portfolio::from_file(&cli.portfolio)
        .await
        .unwrap();

    match &cli.command {
        Some(Commands::Invest { amount, currency }) => {
            let amount = Amount::new(
                Currency::from_str(currency)
                    .expect(format!("Unknown invest currency: {}!", &currency).as_str()),
                *amount,
            );
            let change_request = portfolio.balance(amount);
            println!("{}", &change_request);
        }
        Some(Commands::Show) => {
            println!("{}", &portfolio);
        }
        None => todo!(),
    }
}
