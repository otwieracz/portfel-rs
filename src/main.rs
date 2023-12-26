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
        portfolio: Option<String>,
        #[arg(short, long)]
        xtb_accont_id: Option<String>,
    },
    Show {
        #[clap(short, long, value_name = "YAML")]
        portfolio: Option<String>,
    },
    Invest {
        #[clap(short, long, value_name = "YAML")]
        portfolio: Option<String>,
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

fn get_portfolio_file(path: &Option<String>) -> String {
    let file_path = match path {
        Some(path) => path.to_owned(),
        None => { 
            let dirs = directories::ProjectDirs::from("pl", "slawekgonet", "portfel").unwrap();
                dirs.data_dir()
                    .join("portfolio.yaml")
                    .to_str()
                    .unwrap()
                    .to_owned()

        }};
    println!("Using portfolio file: {}", file_path);
    if !std::path::Path::new(&file_path).exists() {
        log::error!("Portfolio file does not exist: {}", file_path);
        std::process::exit(1);
    }
    file_path
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    env_logger::init();

    match &cli.command {
        Some(Commands::Invest {
            amount,
            currency,
            portfolio,
        }) => {
            let portfolio_file = get_portfolio_file(portfolio);
            let key = rpassword::prompt_password("Portfolio key: ").unwrap();
            match portfolio::Portfolio::from_file(&portfolio_file, &key).await {
                Ok(portfolio) => {
                    let amount = Amount::new(
                        Currency::from_str(currency)
                            .expect(format!("Unknown invest currency: {}!", &currency).as_str()),
                        *amount,
                    );
                    let change_request = portfolio.balance(amount);
                    println!(
                        "{}",
                        &change_request
                            .expect("Unable to balance portfolio!")
                            .format(&portfolio)
                    );
                }
                Err(e) => {
                    log::error!("Error reading portfolio file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Show { portfolio }) => {
            let portfolio_file = get_portfolio_file(portfolio);
            let key = rpassword::prompt_password("Portfolio key: ").unwrap();

            match portfolio::Portfolio::from_file(&portfolio_file, &key).await {
                Ok(portfolio) => {
                    println!("{}", &portfolio);
                }
                Err(e) => {
                    log::error!("Error reading portfolio file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Init {
            portfolio,
            xtb_accont_id: xtb_account_id,
        }) => {
            let portfolio_file = get_portfolio_file(portfolio);
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
                .to_file(&portfolio_file)
                .await
            {
                Ok(filename) => {
                    println!("Initialized portfolio file: {}", filename);
                }
                Err(e) => {
                    log::error!("Error writing portfolio file: {}", e);
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
            log::warn!("No command specified!");
            std::process::exit(1);
        }
    }
}
