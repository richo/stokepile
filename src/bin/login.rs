#[macro_use]
extern crate log;

use clap::App;
use rpassword;

use stokepile::cli;
use stokepile::config;
use stokepile::client;
use std::io::{self, Write};

use tokio;
use stokepile::async_hacks;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Logs into the stokepile web interface for configuration management")
}

#[tokio::main]
async fn main() {
    stokepile::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));

        let base = match cfg {
            Ok(cfg) => {
                cfg.api_base().to_string()
            },
            Err(_) => {
                info!("Error loading config, proceeding with default api base");
                config::DEFAULT_API_BASE.to_string()
            },
        };
        let client = client::StokepileClient::new(&base)?;
        let mut email = String::new();
        let stdin = io::stdin();
        let password;
        println!("Logging into {}", base);
        print!("email: ");
        io::stdout().flush()?;
        stdin.read_line(&mut email)?;
        password = rpassword::prompt_password_stdout("password: ")?;
        println!("Logging in");
        let token = async_hacks::block_on(client.login(email.trim_end(), &password))?;
        println!("Token recieved, saving to ~/{}", config::TOKEN_FILE_NAME);

        // TODO(richo) rewrite config including token
        config::AccessToken::save(&token)?;

        Ok(())
    })
}
