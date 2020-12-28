#![feature(decl_macro, proc_macro_hygiene)]

use stokepile::web::db::run_migrations;
use stokepile::cli::init_dotenv;
use stokepile::web::config_server::configure_rocket;

fn main() {
    stokepile::cli::run(|| {
        init_dotenv()?;
        run_migrations()?;
        configure_rocket().launch();
        Ok(())
    })
}
