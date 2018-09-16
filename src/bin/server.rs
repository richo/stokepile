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
    static ref YOUTUBE_CONFIG: Oauth2Config = {
        info!("Initializing Youtube oauth config");
        Oauth2Config::youtube()
    };
}

static DROPBOX_TOKEN_COOKIE_NAME: &'static str = "dropbox_token";
static YOUTUBE_TOKEN_COOKIE_NAME: &'static str = "youtube_token";

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

#[get("/youtube/auth")]
fn youtube_auth(mut cookies: Cookies) -> Redirect {
    let client = YOUTUBE_CONFIG.client();
    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random);
    cookies.add(Cookie::new("youtube_oauth_state", csrf_state.secret().to_string()));

    Redirect::to(authorize_url.as_str().to_string())
}

#[derive(Serialize)]
struct OauthState<'a> {
    success: bool,
    service: &'a str,
}
#[get("/dropbox/finish?<resp>")]
fn dropbox_finish(mut ctx: Ctx<OauthState>, resp: Oauth2Response, mut cookies: Cookies) -> Template {
    let mut local = OauthState {
        success: false,
        service: "dropbox",
    };

    if cookies.get("dropbox_oauth_state").map(|c| c.value()) != Some(&resp.state) {
        warn!("Something went wrong with your oauth state");
    } else {
        cookies.add(Cookie::build(DROPBOX_TOKEN_COOKIE_NAME, resp.code).path("/").finish());
        local.success = true;
    }

    ctx.local = Some(local);
    return Template::render("oauth_finish", &ctx);
}

#[get("/youtube/finish?<resp>")]
fn youtube_finish(mut ctx: Ctx<OauthState>, resp: Oauth2Response, mut cookies: Cookies) -> Template {
    let mut local = OauthState {
        success: false,
        service: "youtube",
    };

    if cookies.get("youtube_oauth_state").map(|c| c.value()) != Some(&resp.state) {
        warn!("Something went wrong with your oauth state");
    } else {
        cookies.add(Cookie::build(YOUTUBE_TOKEN_COOKIE_NAME, resp.code).path("/").finish());
        local.success = true;
    }

    ctx.local = Some(local);
    return Template::render("oauth_finish", &ctx);
}

#[get("/config.json")]
fn get_config(cookies: Cookies) -> Result<Json<Config>, BadRequest<&'static str>> {
    if let Some(dbx_token) = cookies.get(DROPBOX_TOKEN_COOKIE_NAME) {
        let mut config = Config::build(dbx_token.value().to_string());
        Ok(Json(config))
    } else {
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
    let _ = YOUTUBE_CONFIG.client();

    rocket::ignite()
        .mount("/", routes![index,
               dropbox_auth, dropbox_finish,
               youtube_auth, youtube_finish,
               get_config,
        ])
        .mount("/static", StaticFiles::from("web/static"))
        .attach(Template::fairing())
        .launch();
}
