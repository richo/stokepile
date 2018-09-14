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

use archiver::config::Config;

#[get("/config")]
fn get_config() -> Result<Json<Config>, Error> {
    let config = Config::from_file("archiver.toml.example")?;
    info!("Butts");
    Ok(Json(config))
}

fn init_logging() {
    if let None = env::var_os("RUST_LOG") {
        env::set_var("RUST_LOG", "INFO");
    }
    pretty_env_logger::init();
}

fn main() {
    init_logging();
    rocket::ignite().mount("/", routes![get_config]).launch();
}
