#![feature(decl_macro, proc_macro_hygiene)]

use stokepile::web::db::run_migrations;
use stokepile::web::configure_rocket;

fn main() {
    stokepile::cli::run(|| {
        run_migrations()?;
        configure_rocket().launch();
        Ok(())
    })
}
