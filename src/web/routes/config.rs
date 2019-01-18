use rocket::get;
use rocket::response::content::Content;
use rocket::http::ContentType;
use rocket::response::{Flash, Redirect};

use crate::config::{Config, DeviceConfig};
use crate::web::auth::AuthenticatedUser;
use crate::web::db::DbConn;
use crate::web::oauth::Oauth2Provider;

#[get("/config")]
pub fn get_config(user: AuthenticatedUser, conn: DbConn) -> Result<Content<String>, Flash<Redirect>> {
    let mut config = Config::build();

    let integrations = user.user().integrations(&*conn).map_err(|e| {
        Flash::error(
            Redirect::to("/"),
            format!("Error connecting to the DB: {}", e),
        )
    })?;
    let mut integrations = integrations.iter();
    for provider in Oauth2Provider::providers() {
        let name = provider.name();

        if let Some(integration) = integrations.find(|ref x| x.provider == name) {
            let token = integration.access_token.to_string();
            match name {
                "dropbox" => config = config.dropbox(token),
                "vimeo" => config = config.vimeo(token),
                name => {
                    warn!("Unknown integration: {}", name);
                }
            }
        }
    }

    let devices = user.user().devices(&*conn).map_err(|e| {
        Flash::error(
            Redirect::to("/"),
            format!("Error connecting to the DB: {}", e),
        )
    })?;
    for device in devices {
        match device.into() {
            DeviceConfig::Gopro(gopro) => config = config.gopro(gopro),
            DeviceConfig::Flysight(flysight) => config = config.flysight(flysight),
            DeviceConfig::MassStorage(mass_storage) => config = config.mass_storage(mass_storage),
            DeviceConfig::UnknownDevice(kind) => warn!("Unknown device kind: {}", kind),
        }
    }

    match config.finish() {
        Ok(config) => Ok(Content(
            ContentType::new("application", "toml"),
            config.to_toml(),
        )),
        Err(error) => Err(Flash::error(
            Redirect::to("/"),
            format!(
                "There was a problem generating configuration for you: {}",
                error
            ),
        )),
    }
}
