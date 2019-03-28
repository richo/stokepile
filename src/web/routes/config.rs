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

    if let Some(staging) = user.user().staging() {
        config = config.staging(staging);
    }

    fn build_flash_error(error: Error) -> Flash<Redirect> {
        Flash::error(
            Redirect::to("/"),
            format!(
                "There was a problem generating configuration for you: {}",
                error
            ),
            )
    }

    match config.finish().map(|c| c.to_toml()) {
        Ok(Ok(config)) => Ok(Content(
            ContentType::new("application", "toml"),
            config,
        )),
        Ok(Err(error)) => Err(build_flash_error(error)),
        Err(error) => Err(build_flash_error(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;

    use rocket::http::{Header, Status};
    use crate::web::models::NewIntegration;
    use crate::web::models::NewKey;
    use crate::web::models::NewDevice;

    use crate::config::{Config, FlysightConfig};

    client_for_routes!(get_config => client);

    #[test]
    fn test_anon_get_config() {
        let client = client();
        let req = client.get("/config");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[test]
    fn test_get_config_with_no_integrations() {
        let client = client();

        create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let req = client.get("/config");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
    }

    #[test]
    fn test_get_config() {
        let client = client();

        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        {
            let conn = db_conn(&client);

            NewIntegration::new(&user, "dropbox", "test_oauth_token")
                .create(&*conn)
                .unwrap();
        }

        let req = client.get("/config");

        let mut response = req.dispatch();
        assert_eq!(response.status(), Status::Ok);
        let config: Config =
            response.body_string().expect("Didn't recieve a body").parse().unwrap();
        let backend_names: Vec<_> = config.backends().iter().map(|b| b.name()).collect();
        assert_eq!(&backend_names, &["dropbox"]);
    }

    #[test]
    fn test_get_config_with_api_token() {
        let client = client();

        let user = create_user(&client, "test@email.com", "p@55w0rd");

        {
            let conn = db_conn(&client);

            NewIntegration::new(&user, "dropbox", "test_oauth_token")
                .create(&*conn)
                .unwrap();
        }

        let token = {
            let conn = db_conn(&client);

            NewKey::new(&user).create(&*conn).unwrap().token
        };

        let req = client
            .get("/config")
            .header(Header::new("Authorization", format!("Bearer: {}", token)));

        let mut response = req.dispatch();
        assert_eq!(response.status(), Status::Ok);
        let config: Config =
            response.body_string().expect("Didn't recieve a body").parse().unwrap();
        let backend_names: Vec<_> = config.backends().iter().map(|b| b.name()).collect();
        assert_eq!(&backend_names, &["dropbox"]);
    }

    #[test]
    fn test_get_config_with_invalid_api_token() {
        let client = client();

        let user = create_user(&client, "test@email.com", "p@55w0rd");

        {
            let conn = db_conn(&client);

            NewIntegration::new(&user, "dropbox", "test_oauth_token")
                .create(&*conn)
                .unwrap();
        }

        let token = {
            let conn = db_conn(&client);

            NewKey::new(&user).create(&*conn).unwrap()
        };

        {
            let conn = db_conn(&client);
            token.expire(&*conn).unwrap();
        }

        let req = client.get("/config").header(Header::new(
            "Authorization",
            format!("Bearer: {}", token.token),
        ));

        let response = req.dispatch();
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[test]
    fn test_get_config_with_devices() {
        let client = client();

        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        {
            let conn = db_conn(&client);

            NewIntegration::new(&user, "dropbox", "test_oauth_token")
                .create(&*conn)
                .unwrap();
        }

        {
            let conn = db_conn(&client);

            NewDevice::new(&user, "gopro", "ptp", "serial")
                .create(&*conn)
                .unwrap();
            NewDevice::new(&user, "fake", "bogus", "serial")
                .create(&*conn)
                .unwrap();
            NewDevice::new(&user, "sd card", "mass_storage", "serial")
                .create(&*conn)
                .unwrap();
        }

        let req = client.get("/config");

        let mut response = req.dispatch();
        assert_eq!(response.status(), Status::Ok);
        let config: Config =
            response.body_string().expect("Didn't recieve a body").parse().unwrap();
        let backend_names: Vec<_> = config.backends().iter().map(|b| b.name()).collect();
        assert_eq!(&backend_names, &["dropbox"]);

        let empty_flysights: Vec<FlysightConfig> = vec![];
        assert_eq!(config.flysights(), &empty_flysights);
        assert_eq!(config.mass_storages().len(), 1);
        assert_eq!(config.gopros().len(), 1);
    }
}
