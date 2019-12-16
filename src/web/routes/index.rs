use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::{Context, PossibleIntegration};

use rocket::request::FlashMessage;
use rocket_contrib::templates::Template;

use crate::messages::Oauth2Provider;

use crate::web::models::{Device, Key, User};

// TODO(richo) This might want to live elsewhere?
#[derive(Serialize, Debug)]
pub struct MediaView {
    pub integrations: Vec<PossibleIntegration>,
    pub devices: Vec<Device>,
    pub keys: Vec<Key>,
}

#[get("/")]
pub fn index(user: Option<WebUser>, conn: DbConn, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let mut possible_integrations = vec![];
    let mut devices = vec![];
    let mut keys = vec![];

    if let Some(user) = &user {
        if let Ok(integrations) = user.user.integrations(&*conn) {

            for provider in Oauth2Provider::providers() {
                let name = provider.name();

                let mut integrations = integrations.iter();
                let configured_integration = integrations.find(|ref x| x.provider == name);

                possible_integrations.push(PossibleIntegration {
                    id: configured_integration.map(|i| i.id),
                    name: provider.name(),
                    display_name: provider.display_name(),
                    connected: configured_integration.is_some(),
                });
            }
        }
        devices = user.user.devices(&*conn).unwrap();
        keys = user.user.keys(&*conn).unwrap();
    }

    let view_data = MediaView {
        integrations: possible_integrations,
        devices,
        keys,
    };

    let context = Context::media(view_data)
        .set_user(user)
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("index", context)
}

#[get("/privacy")]
pub fn privacy() -> Template {
    let context = Context::other();
    Template::render("privacy", context)
}

#[catch(404)]
pub fn not_found() -> Template {
    let context = Context::other();
    Template::render("404", context)
}
