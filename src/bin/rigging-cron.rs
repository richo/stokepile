#[macro_use]
extern crate log;

use chrono::prelude::*;
use chrono::Duration;

use stokepile::web::db::db_connection;
use stokepile::web::models::{User, NewInvite};

use clap::{App, Arg, SubCommand};

fn cli_opts<'a, 'b>(base: App<'a, 'b>) -> App<'a, 'b> {
    base.about("Runs periodic tasks for the rigging module")
}

fn main() {
    stokepile::cli::run(cli_opts, |matches| {
        let conn = db_connection()?;
        let users = User::all(&conn)?;

        let filter_date = Utc::now()
            .naive_utc()
            .checked_add_signed(Duration::days(30))
            .expect("30 days from now")
            .date();

        let equipment_by_user = users.iter().filter_map(|u| {
            let equipment: Vec<_> = u.equipment(&conn)
                .expect(&format!("Couldn't load equipment for {}", &u.id))
                .into_iter()
                .map(|e| e.to_assembly(&conn)
                    .expect("Couldn't load assembly"))
                .filter(|asm| asm.due_before(filter_date))
                .collect();
            if equipment.len() > 0 {
                Some((u, equipment))
            } else {
                None
            }
        });

        info!("Due rigs");
        for (u, equipment) in equipment_by_user {
            info!("{}", u.id);
            for e in equipment.iter() {
                info!("  {:?}", &e);
            }
        }

        Ok(())
    })
}
