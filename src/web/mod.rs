use rocket::{Rocket, Route};
use rocket::config::Environment;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

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

lazy_static! {
    pub static ref ROCKET_ENV: Environment = Environment::active().expect("Could not get ROCKET_ENV.");
}

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

fn configure_rocket(routes: Vec<Route>) -> Rocket {
    rocket::ignite()
        .mount(
            "/",
            routes
        )
        .mount("/static", StaticFiles::from("web/static"))
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
