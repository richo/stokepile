#![feature(decl_macro, proc_macro_hygiene)]

use dotenv;

use archiver::web::configure_rocket;

fn main() {
    archiver::cli::run(|| {
        dotenv::dotenv().ok();
        configure_rocket().launch();
        Ok(())
    })
}
