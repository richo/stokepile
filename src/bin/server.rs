#![feature(decl_macro, proc_macro_hygiene)]

use archiver::cli::init_dotenv;
use archiver::web::configure_rocket;

fn main() {
    archiver::cli::run(|| {
        init_dotenv()?;
        configure_rocket().launch();
        Ok(())
    })
}
