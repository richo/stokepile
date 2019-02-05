use rocket::Rocket;
use rocket::config::Environment;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use crate::web::db::init_pool;

pub mod auth;
pub mod context;
pub mod db;
pub mod models;
pub mod oauth;
pub mod routes;
pub mod schema;

lazy_static! {
    pub static ref ROCKET_ENV: Environment = Environment::active().expect("Could not get ROCKET_ENV.");
}

pub fn configure_rocket() -> Rocket {
    rocket::ignite()
        .manage(init_pool(false))
        .mount(
            "/",
            routes![
                routes::config::get_config,

                routes::sessions::get_signin,
                routes::sessions::post_signin,
                routes::sessions::signin_json,
                routes::sessions::signout,
                routes::sessions::expire_key,

                routes::settings::get_settings,
                routes::settings::post_settings,

                routes::index::index,

                routes::integrations::connect_integration,
                routes::integrations::disconnect_integration,
                routes::integrations::finish_integration,

                routes::devices::create_device,
                routes::devices::delete_device,
            ],
        )
        .mount("/static", StaticFiles::from("web/static"))
        .attach(Template::fairing())
}

pub fn create_test_rocket(routes: Vec<rocket::Route>) -> Rocket {
    rocket::ignite()
        .manage(init_pool(true))
        .mount(
            "/",
            routes,
        )
}

#[cfg(test)]
pub mod test_helpers;
