#[macro_use]
extern crate log;

use clap::App;

use archiver::cli;
use archiver::config;
use archiver::client;

use std::fs::File;
use std::io::Write;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Fetches configuration from upstream")
}

fn main() {
    archiver::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"));

        let base = match cfg {
            Ok(cfg) => {
                cfg.api_base().to_string()
            },
            Err(_) => {
                info!("Error loading config, proceeding with default api base");
                config::DEFAULT_API_BASE.to_string()
            },
        };
        let token = config::AccessToken::load()?;
        info!("Creating client");
        let client = client::ArchiverClient::new(&base)?;
        info!("Fetching config from {}", &base);
        let config = client.fetch_config(token)?;
        let filename = matches.value_of("config").unwrap_or("archiver.toml");
        let mut fh = File::create(&filename)?;
        fh.write(config.to_toml().as_bytes())?;
        info!("Wrote config to {}", &filename);

        Ok(())
    })
}

