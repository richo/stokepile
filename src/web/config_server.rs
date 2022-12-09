use rocket::Rocket;
use crate::web::routes;
use crate::web::db::{init_pool, DbConn};

pub fn configure_rocket()<P> -> Rocket<P> {
    super::configure_rocket(
        routes![
        routes::admin::index,
        routes::admin::create_invite,
        routes::admin::users,

        routes::config::get_config,

        routes::sessions::get_signin,
        routes::sessions::post_signin,
        routes::sessions::signin_json,
        routes::sessions::signout,
        routes::sessions::expire_key,
        routes::sessions::refresh_token,

        routes::settings::get_settings,
        routes::settings::post_settings,

        routes::healthcheck::healthcheck,

        routes::index::index,
        routes::index::privacy,

        routes::help::help,
        // TODO(richo) Remove this when the beta is done.
        routes::help::beta,

        routes::notifications::notification_send,

        routes::integrations::connect_integration,
        routes::integrations::disconnect_integration,
        routes::integrations::finish_integration,

        routes::devices::create_device,
        routes::devices::delete_device,
        ]
    )
    .manage(init_pool(false))
}
