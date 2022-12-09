use rocket::{Rocket, Route};
use rocket::fs::FileServer;
use rocket_dyn_templates::Template;

use crate::web::db::{init_pool, DbConn};

use logging::RequestLogger;

pub mod auth;
pub mod context;
pub mod db;
pub mod models;
pub mod oauth;
pub mod routes;
pub mod schema;
mod form_hacks;
mod logging;
mod range;

pub use self::range::RangeResponder;

pub mod media_server;
pub mod config_server;

handlebars_helper!(maybe_selected: |field: str, active: str| {
    if field== active {
        format!("selected")
    } else {
        format!("")
    }});

handlebars_helper!(maintainer_info: |kind: str| {
    match kind {
        "email" => {
            "richo@psych0tik.net"
        },
        _ => {
            panic!("Unknown info kind: {}", &kind)
        },
    }
});

fn configure_rocket<T>(routes: Vec<Route>) -> Rocket<T> {
    rocket::build()
        .mount(
            "/",
            routes
        )
        .mount("/static", FileServer::from("web/static"))
        .register(catchers![routes::index::not_found])
        .attach(RequestLogger::new())
        .attach(Template::custom(|engines| {
            engines.handlebars.register_helper("maybe_selected", Box::new(maybe_selected));
            engines.handlebars.register_helper("maintainer_info", Box::new(maintainer_info));
            engines.handlebars.set_strict_mode(true);
        }))
}

pub fn global_state(conn: &DbConn) -> diesel::prelude::QueryResult<models::GlobalSetting> {
    models::GlobalSetting::get(conn)
}

#[cfg(test)]
pub mod test_helpers;
