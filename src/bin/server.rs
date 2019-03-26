#![feature(decl_macro, proc_macro_hygiene)]

use dotenv;

use archiver::web::db::run_migrations;
use archiver::web::configure_rocket;

fn main() {
    archiver::cli::run(|| {
        dotenv::dotenv().ok();
        run_migrations()?;
        configure_rocket().launch();
        Ok(())
    })
}
