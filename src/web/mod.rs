use rocket::Rocket;
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
mod logging;

pub mod links {
    pub fn equipment_link_for_customer(customer_id: i64) -> String {
        format!("/rigging/equipment?customer_id={}", &customer_id)
    }
}

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

handlebars_helper!(rigging_equipment_link_for_customer: |customer_id: i64| {
    links::equipment_link_for_customer(customer_id)
});

handlebars_helper!(rigging_customer_link_for_equipment: |customer_id: i64| {
    format!("/rigging/customer/{}", &customer_id)
});

handlebars_helper!(rigging_equipment_detail: |equipment_id: i64| {
    format!("/rigging/equipment/{}", &equipment_id)
});

pub fn configure_rocket() -> Rocket {
    rocket::ignite()
        .manage(init_pool(false))
        .mount(
            "/",
            routes![
                routes::sessions::get_signin,
                routes::sessions::post_signin,
                routes::sessions::signin_json,
                routes::sessions::signout,
                routes::sessions::expire_key,
                routes::sessions::refresh_token,

                routes::healthcheck::healthcheck,

                routes::index::index,
                routes::index::privacy,

                routes::help::help,
                // TODO(richo) Remove this when the beta is done.
                routes::help::beta,
            ],
        )
        .mount(
            "/admin",
            routes![
                routes::admin::index,
                routes::admin::create_invite,
                routes::admin::users,


            ]
        )
        .mount(
            "/media",
            routes![
                routes::media::index,

                routes::config::get_config,

                routes::settings::get_settings,
                routes::settings::post_settings,

                routes::notifications::notification_send,

                routes::integrations::connect_integration,
                routes::integrations::disconnect_integration,
                routes::integrations::finish_integration,

                routes::devices::create_device,
                routes::devices::delete_device,
            ]
        )
        .mount(
            "/rigging",
            routes![
                routes::rigging::index,
                routes::rigging::customers,
                routes::rigging::customer_create,
                routes::rigging::customer_detail,
                routes::rigging::equipment,
                routes::rigging::equipment_create,
                routes::rigging::equipment_detail,
                routes::rigging::service_bulletins,

            ]
        )
        .mount("/static", StaticFiles::from("web/static"))
        .register(catchers![routes::index::not_found])
        .attach(RequestLogger::new())
        .attach(template_engine())
}

pub fn create_test_rocket(routes: Vec<rocket::Route>) -> Rocket {
    rocket::ignite()
        .manage(init_pool(true))
        .mount(
            "/",
            routes,
        )
        .attach(RequestLogger::new())
        .attach(template_engine())
}

fn template_engine() -> impl rocket::fairing::Fairing {
    Template::custom(|engines| {
        engines.handlebars.register_helper("maybe_selected", Box::new(maybe_selected));
        engines.handlebars.register_helper("maintainer_info", Box::new(maintainer_info));
        engines.handlebars.register_helper("rigging_equipment_link_for_customer", Box::new(rigging_equipment_link_for_customer));
        engines.handlebars.register_helper("rigging_customer_link_for_equipment", Box::new(rigging_customer_link_for_equipment));
        engines.handlebars.register_helper("rigging_equipment_detail", Box::new(rigging_equipment_detail));
        engines.handlebars.set_strict_mode(true);
    })
}

pub fn global_state(conn: &DbConn) -> diesel::prelude::QueryResult<models::GlobalSetting> {
    models::GlobalSetting::get(conn)
}

#[cfg(test)]
pub mod test_helpers;
