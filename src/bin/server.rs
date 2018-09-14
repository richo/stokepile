#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate log;

extern crate pretty_env_logger;
extern crate failure;
extern crate rocket;
extern crate rocket_contrib;

extern crate archiver;

use rocket_contrib::Json;
use failure::Error;
use std::env;
use std::process;

use archiver::config;

#[get("/")]
fn index() -> &'static str {
        "Hello, world!"
}

fn run() -> Result<(), Error> {
    rocket::ignite().mount("/", routes![index]).launch();
    Ok(())
}

fn init_logging() {
    if let None = env::var_os("RUST_LOG") {
        env::set_var("RUST_LOG", "INFO");
    }
    pretty_env_logger::init();
}

fn main() {
    init_logging();
    if let Err(e) = run() {
        error!("Error running archiver");
        error!("{:?}", e);
        if env::var("RUST_BACKTRACE").is_ok() {
            error!("{:?}", e.backtrace());
        }
        process::exit(1);
    }
}
