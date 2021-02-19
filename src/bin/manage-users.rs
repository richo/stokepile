#[macro_use]
extern crate log;

use stokepile::cli::self;
use stokepile::web::db::db_connection;
use stokepile::web::models::{User, NewInvite};

use clap::{App, Arg, SubCommand};

fn cli_opts<'a, 'b>(base: App) -> App<'a, 'b> {
    base.subcommand(SubCommand::with_name("invite")
        .about("Creates an invite code")
        .arg(Arg::with_name("EMAIL")
            .help("Email address to invite")
            .required(true)
            .index(1)))

        .subcommand(SubCommand::with_name("promote")
            .about("Promotes a user to admin")
            .arg(Arg::with_name("EMAIL")
                .help("Email address to promote")
                .required(true)
                .index(1)))

        .subcommand(SubCommand::with_name("demote")
            .about("Demotes an admin to a regular user")
            .arg(Arg::with_name("EMAIL")
                .help("Email address to demote")
                .required(true)
                .index(1)))
}

fn main() {
    stokepile::cli::run(cli_opts, |matches| {
        let conn = db_connection()?;

        if let Some(matches) = matches.subcommand_matches("invite") {
            let email = matches.value_of("EMAIL").unwrap();

            let invite = NewInvite::new(email).create(&conn);
            info!("Created invite: {:?}", invite);
        } else if let Some(matches) = matches.subcommand_matches("promote") {
            let email = matches.value_of("EMAIL").unwrap();

            let user = User::by_email(&conn, email)?;
            let result = user.promote(&conn);
            info!("Promoted user: {:?}", result);
        } else if let Some(matches) = matches.subcommand_matches("demote") {
            let email = matches.value_of("EMAIL").unwrap();

            let user = User::by_email(&conn, email)?;
            let result = user.demote(&conn);
            info!("Demoted user: {:?}", result);
        }

        Ok(())
    })
}
