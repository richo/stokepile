use rocket::{Rocket, Route};
use rocket::config::Environment;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use crate::web::db::{init_pool, DbConn};

use logging::RequestLogger;

pub mod auth;
pub mod context;
pub mod db;
pub mod form_hacks;
pub mod forms;
pub mod models;
pub mod oauth;
pub mod routes;
pub mod schema;
mod form_hacks;
mod logging;
mod range;

pub use range::RangeResponder;

pub mod media_server;
pub mod config_server;

pub mod links {
    pub fn equipment_link_for_customer(customer_id: i64) -> String {
        format!("/rigging/equipment?customer_id={}", &customer_id)
    }

    pub fn equipment_detail_link(equipment_id: i64) -> String {
        format!("/rigging/equipment/{}", &equipment_id)
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

fn configure_rocket(routes: Vec<Route>) -> Rocket {
    rocket::ignite()
        .mount(
            "/",
            routes
        )
        .mount("/static", StaticFiles::from("web/static"))
        .register(catchers![routes::index::not_found])
        .attach(RequestLogger::new())
        .attach(template_engine())
}

handlebars_helper!(rigging_equipment_link_for_customer: |customer_id: i64| {
    links::equipment_link_for_customer(customer_id)
});

handlebars_helper!(rigging_customer_link_for_equipment: |customer_id: i64| {
    format!("/rigging/customer/{}", &customer_id)
});

handlebars_helper!(rigging_equipment_detail: |equipment_id: i64| {
    links::equipment_detail_link(equipment_id)
});

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
