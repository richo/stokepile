#![feature(plugin, decl_macro, custom_derive)]
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
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
extern crate oauth2;

extern crate archiver;

use rocket::http::{Cookie, Cookies, Status};
use rocket::response::{Response, Responder, Redirect};
use rocket::response::status::{BadRequest};
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

#[derive(Serialize)]
struct NoLocal {}

#[get("/")]
fn index(ctx: Ctx<NoLocal>) -> Template {
    Template::render("index", &ctx)
}

#[get("/dropbox/auth")]
fn dropbox_auth(mut cookies: Cookies) -> Redirect {
    let client = DROPBOX_CONFIG.client();
    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random);
    cookies.add(Cookie::new("dropbox_oauth_state", csrf_state.secret().to_string()));

    Redirect::to(authorize_url.as_str().to_string())
}

#[derive(Serialize)]
struct DropboxOauthState {
    success: bool,
}
#[get("/dropbox/finish?<resp>")]
fn dropbox_finish(mut ctx: Ctx<DropboxOauthState>, resp: Oauth2Response, mut cookies: Cookies) -> Template {
    let mut local = DropboxOauthState {
        success: false,
    };

    info!("cookies: {:?}", cookies.iter().collect::<Vec<_>>());

    if cookies.get("dropbox_oauth_state").map(|c| c.value()) != Some(&resp.state) {
        warn!("Something went wrong with your oauth state");
    } else {
        cookies.add(Cookie::new("dropbox_token", resp.code));
        local.success = true;
    }

    ctx.local = Some(local);
    return Template::render("dropbox_finish", &ctx);
}

#[get("/config.json")]
fn get_config(cookies: Cookies) -> Result<Json<Config>, BadRequest<&'static str>> {
    if let Some(dbx_token) = cookies.get("dropbox_token") {
        let mut config = Config::build(dbx_token.value().to_string());
        Ok(Json(config))
    } else {
        info!("cookies: {:?}", cookies.iter().collect::<Vec<_>>());
        Err(BadRequest(Some("dropbox account not linked")))
    }
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
               get_config,
        ])
        .mount("/static", StaticFiles::from("web/static"))
        .attach(Template::fairing())
        .launch();
}
