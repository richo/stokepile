#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate log;

extern crate serde;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

extern crate pretty_env_logger;
extern crate failure;
extern crate rocket;
extern crate rocket_contrib;

extern crate archiver;

use rocket_contrib::{Template, Json};
use rocket_contrib::static_files::StaticFiles;
use failure::Error;
use std::env;
use std::process;

use archiver::config::Config;
use archiver::web::Ctx;


#[get("/")]
fn index(ctx: Ctx) -> Template {
    Template::render("index", &ctx)
}

#[get("/authorize/dropbox")]
fn authorize_dropbox(ctx: Ctx) -> Template {
    Template::render("index", &ctx)
}

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
    rocket::ignite()
        .mount("/", routes![index, authorize_dropbox, get_config])
        .mount("/static", StaticFiles::from("web/static"))
        .attach(Template::fairing())
        .launch();
}
