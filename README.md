# Portfolio Management CLI

This application is tailored for effortless management of a stable, long-term investment portfolio, automatically computing and suggesting the optimal investment amounts to maintain a predefined balance across a set of positions.

It is purpose-built for a specific scenario, streamlining the management of a stable, long-term investment portfolio with a focus on simplicity, and may not be suitable for more complex use cases.

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Commands](#commands)
- [Examples](#examples)
- [Contributing](#contributing)
- [License](#license)

## Installation

Make sure you have Rust installed on your system. You can then build the project using:

```bash
cargo build --release
```

The binary will be available in the `target/release` directory.

## Usage

See the help message for up-to-date usage information.

Note: The majority of actions are intended to be undertaken in the YAML file. 
The portfolio file is designed to be manually edited.

Also, please be aware that the 'invest' command is a simulation command and does not
update the YAML portfolio file. It displays the suggested investment amount and
currency to keep the portfolio balanced without modifying the existing portfolio data.

```bash
portfolio-cli --help
```


### Initialize Portfolio

Initialize a new portfolio with an optional XTB account configuration:

```bash
portfolio-cli init --portfolio <PATH_TO_PORTFOLIO_FILE> --xtb_account_id <XTB_ACCOUNT_ID>
```

If you do not provide an XTB account ID, you will be prompted to enter your XTB account credentials and portfolio secret. The password will be encrypted and stored in the portfolio.

After initialization, you should manually edit the portfolio file to add your investments.
- Create investment groups, and then add investments to the group.
- If position is in a group with XTB account, `amount` is optional and will be read from the broker.

### Display Portfolio Details

Display details of an existing portfolio:

(this is hand-crafted example, there might be discrepancies with the actual output)

```bash
portfolio-cli show --portfolio <PATH_TO_PORTFOLIO_FILE>
Portfolio key: <type password>
Total value: 10000.00 USD
Positions:
- [SPXS.UK ] Invesco S&P 500 UCITS ETF            :   4500.00 USD [0.48 ~ 0.45]
- [IMAE.NL ] iShares Core MSCI Europe UCITS ETF   :   1831.50 EUR [0.18 ~ 0.17]
- [EIMI.UK ] iShares Core MSCI Emerging Markets I :    800.00 USD [0.09 ~ 0.08]
- [SJPA.UK ] iShares Core MSCI Japan IMI UCITS ET :    627.50 GBP [0.05 ~ 0.05]
- [CASH_USD] Cash (USD)                           :   2500.00 USD [0.20 ~ 0.25]
```

### Simulate an Investment

Simulate an investment in the portfolio, and display the suggested investment amount to each position and total amount to invest per group.

**The goal is to make the portfolio balanced according to the target percentages specified for each position in the portfolio file.**

(again, this is hand-crafted example, there might be discrepancies with the actual output)

```bash
portfolio-cli invest --portfolio <PATH_TO_PORTFOLIO_FILE> --amount <INVESTMENT_AMOUNT> --currency <CURRENCY>

 
Portfolio key: <type password}
Change requests:
- [SPXS.UK ] Invesco S&P 500 UCITS ETF            :   4500.00 USD -[+    100.00 USD]>   4700.00 USD
- [IMAE.NL ] iShares Core MSCI Europe UCITS ETF   :   1831.50 EUR -[+    200.00 EUR]>   2031.50 EUR
- [EIMI.UK ] iShares Core MSCI Emerging Markets I :    800.00 USD -[+    300.00 USD]>   1100.00 USD
- [SJPA.UK ] iShares Core MSCI Japan IMI UCITS ET :    627.50 GBP -[+    400.00 GBP]>   1027.50 GBP
- [CASH_USD] Cash (USD)                           :   2500.00 USD -[+    500.00 USD]>   3000.00 USD

Change per group:
- xtb_eur         : +    200.00 EUR
- xtb_usd         : +    901.42 USD
- bank_acc_usd    : +    500.00 PLN
```

### Encrypt Password

Encrypt a password for storing in the portfolio:

```bash
portfolio-cli encrypt-password
```

## Contributing

If you find a bug or have suggestions for improvement, feel free to open an issue or submit a pull request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.