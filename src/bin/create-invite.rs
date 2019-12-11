use stokepile::cli::{self, init_dotenv};
use stokepile::web::db::db_connection;
use stokepile::web::models::NewInvite;

use clap::{App, Arg};

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Creates an invite code")
        .arg(Arg::with_name("EMAIL")
            .help("Email address to invite")
            .required(true)
            .index(1))
}

fn main() {
    stokepile::cli::run(|| {
        init_dotenv()?;

        let matches = cli_opts().get_matches();
        let email = matches.value_of("EMAIL").unwrap();
        let conn = db_connection()?;

        let invite = NewInvite::new(email).create(&conn);

        Ok(())
    })
}
