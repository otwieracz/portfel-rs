use crate::{amount::Amount, amount::Currency};
use clap::{Parser, Subcommand};

mod amount;
mod crypt;
mod error;
mod fx;
mod portfolio;
mod xtb;

#[derive(Subcommand)]
enum Commands {
    EncryptPassword,
    Init {
        #[clap(short, long, value_name = "YAML")]
        portfolio: String,
        #[arg(short, long)]
        xtb_accont_id: Option<String>,
    },
    Show {
        #[clap(short, long, value_name = "YAML")]
        portfolio: String,
    },
    Invest {
        #[clap(short, long, value_name = "YAML")]
        portfolio: String,
        #[arg(short, long)]
        amount: f64,
        #[arg(short, long)]
        currency: String,
    },
}

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Invest {
            amount,
            currency,
            portfolio,
        }) => {
            let key = rpassword::prompt_password("Portfolio key: ").unwrap();
            match portfolio::Portfolio::from_file(&portfolio, &key).await {
                Ok(portfolio) => {
                    let amount = Amount::new(
                        Currency::from_str(currency)
                            .expect(format!("Unknown invest currency: {}!", &currency).as_str()),
                        *amount,
                    );
                    let change_request = portfolio.balance(amount);
                    println!("{}", &change_request);
                }
                Err(e) => {
                    println!("Error reading portfolio file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Show { portfolio }) => {
            let key = rpassword::prompt_password("Portfolio key: ").unwrap();

            match portfolio::Portfolio::from_file(&portfolio, &key).await {
                Ok(portfolio) => {
                    println!("{}", &portfolio);
                }
                Err(e) => {
                    println!("Error reading portfolio file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Init {
            portfolio,
            xtb_accont_id: xtb_account_id,
        }) => {
            let (xtb_config, xtb_account) = if let Some(xtb_account_id) = xtb_account_id {
                let key = rpassword::prompt_password("Portfolio key: ").unwrap();
                let xtb_password = rpassword::prompt_password("XTB password: ").unwrap();
                let xtb_config = Some(xtb::XtbConfig::new("xapi.xtb.com".to_owned(), 5112));
                let xtb_account = Some(
                    xtb::XtbAccount::new(xtb_account_id.clone(), None, Some(xtb_password))
                        .encrypt(key.as_str())
                        .expect("Failed to encrypt password!"),
                );
                (xtb_config, xtb_account)
            } else {
                (None, None)
            };

            match portfolio::Portfolio::example(xtb_config, xtb_account)
                .to_file(&portfolio)
                .await
            {
                Ok(filename) => {
                    println!("Initialized portfolio file: {}", filename);
                }
                Err(e) => {
                    println!("Error writing portfolio file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::EncryptPassword) => {
            let password = rpassword::prompt_password("Password to encrypt: ").unwrap();
            let key = rpassword::prompt_password("Portfolio key: ").unwrap();
            let encrypted = crypt::encrypt_text(&password, &key).unwrap();
            println!("Encrypted password: {}", encrypted);
        }
        None => {
            println!("No command specified!");
            std::process::exit(1);
        }
    }
}
