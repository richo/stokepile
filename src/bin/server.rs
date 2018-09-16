#![feature(plugin, decl_macro, custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate log;

extern crate serde;
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

extern crate pretty_env_logger;
extern crate failure;
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
extern crate oauth2;

extern crate archiver;

use rocket::http::{Cookie, Cookies, Status};
use rocket::response::{Response, Responder, Redirect};
use rocket::request::{Request};
use rocket_contrib::{Template, Json};
use rocket_contrib::static_files::StaticFiles;
use oauth2::prelude::*;
use oauth2::CsrfToken;
use failure::Error;
use std::env;
use std::process;

use archiver::config::Config;
use archiver::web::Ctx;
use archiver::web::oauth2::{Oauth2Config,Oauth2Response};

lazy_static! {
    static ref DROPBOX_CONFIG: Oauth2Config = {
        info!("Initializing Dropbox oauth config");
        Oauth2Config::dropbox()
    };
}

#[get("/")]
fn index(ctx: Ctx) -> Template {
    Template::render("index", &ctx)
}

#[get("/dropbox/auth")]
fn dropbox_auth(mut cookies: Cookies) -> Redirect {
    let client = DROPBOX_CONFIG.client();
    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random);
    cookies.add(Cookie::new("dropbox_oauth_state", csrf_state.secret().to_string()));

    info!("Redirecting to {} ({})", &authorize_url, authorize_url.as_str());
    Redirect::to(authorize_url.as_str().to_string())
}

#[get("/dropbox/finish?<resp>")]
fn dropbox_finish(ctx: Ctx, resp: Oauth2Response) -> Template {
    info!("Got a response from dropbox: {:?}", resp);
    Template::render("dropbox_finish", &ctx)
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
    // Poke these statics to verify they don't panic
    let _ = DROPBOX_CONFIG.client();

    rocket::ignite()
        .mount("/", routes![index,
               dropbox_auth, dropbox_finish,
        ])
        .mount("/static", StaticFiles::from("web/static"))
        .attach(Template::fairing())
        .launch();
}
