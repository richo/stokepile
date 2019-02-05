#![feature(decl_macro, proc_macro_hygiene)]

#[macro_use]
extern crate log;

use dotenv;

#[macro_use]
extern crate rocket;

use rocket::http::RawStr;
use rocket::request::{Form, FromFormValue};
use rocket::response::{Flash, Redirect};
use rocket::Rocket;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use archiver::web::auth::WebUser;
use archiver::web::db::{init_pool, DbConn};
use archiver::web::models::{
    NewDevice,
};
use archiver::web::routes;


#[derive(Debug)]
enum DeviceKind {
    Ptp,
    Flysight,
    MassStorage,
}

impl<'v> FromFormValue<'v> for DeviceKind {
    type Error = String;

    fn from_form_value(form_value: &'v RawStr) -> Result<DeviceKind, Self::Error> {
        let decoded = form_value.url_decode();
        match decoded {
            Ok(ref kind) if kind == "ptp" => Ok(DeviceKind::Ptp),
            Ok(ref kind) if kind == "flysight" => Ok(DeviceKind::Flysight),
            Ok(ref kind) if kind == "mass_storage" => Ok(DeviceKind::MassStorage),
            _ => Err(format!("unknown provider {}", form_value)),
        }
    }
}

impl DeviceKind {
    pub fn name(&self) -> &'static str {
        match self {
            DeviceKind::Ptp => "ptp",
            DeviceKind::Flysight => "flysight",
            DeviceKind::MassStorage => "mass_storage",
        }
    }
}

#[derive(Debug, FromForm)]
struct DeviceForm {
    name: String,
    kind: DeviceKind,
    identifier: String,
}

#[post("/device", data = "<device>")]
fn create_device(
    user: WebUser,
    conn: DbConn,
    device: Form<DeviceForm>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let row = NewDevice::new(
        &user.user,
        &device.name,
        device.kind.name(),
        &device.identifier,
    )
    .create(&*conn)
    .ok();
    match row {
        Some(_) => Ok(Flash::success(
            Redirect::to("/"),
            format!("{} was added to your configuration.", device.kind.name()),
        )),
        None => Err(Flash::error(
            Redirect::to("/"),
            format!(
                "There was a problem adding {} to your configuration.",
                device.kind.name()
            ),
        )),
    }
}

#[derive(Debug, FromForm)]
struct DeleteDeviceForm {
    device_id: i32,
    kind: DeviceKind,
}

#[post("/device/delete", data = "<device>")]
fn delete_device(
    user: WebUser,
    conn: DbConn,
    device: Form<DeleteDeviceForm>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    user.user
        .device_by_id(device.device_id, &*conn)
        .map(|i| i.delete(&*conn))
        .map(|_| {
            Flash::success(
                Redirect::to("/"),
                format!("{} has been removed from your account.", device.kind.name()),
            )
        })
        .map_err(|e| {
            warn!("{}", e);
            Flash::error(
                Redirect::to("/"),
                format!(
                    "{} could not be removed from your account.",
                    device.kind.name()
                ),
            )
        })
}

fn configure_rocket(test_transactions: bool) -> Rocket {
    rocket::ignite()
        .manage(init_pool(test_transactions))
        .mount(
            "/",
            routes![
                routes::config::get_config,

                routes::sessions::get_signin,
                routes::sessions::signin,
                routes::sessions::signin_json,
                routes::sessions::signout,
                routes::sessions::expire_key,

                routes::settings::get_settings,
                routes::settings::post_settings,

                routes::index::index,

                routes::integrations::connect_integration,
                routes::integrations::disconnect_integration,
                routes::integrations::finish_integration,

                create_device,
                delete_device
            ],
        )
        .mount("/static", StaticFiles::from("web/static"))
        .attach(Template::fairing())
}

fn main() {
archiver::cli::run(|| {
    dotenv::dotenv().ok();
    configure_rocket(false).launch();
    Ok(())
})
}

#[cfg(test)]
mod tests {
    use std::env;

    use rocket::http::{ContentType, Header, Status};
    use rocket::local::{Client, LocalResponse};

    use serde_json;

    use archiver::config::{Config, FlysightConfig};
    use archiver::messages;

    use archiver::web::db::DbConn;
    use archiver::web::models::NewDevice;
    use archiver::web::models::NewIntegration;
    use archiver::web::models::NewKey;
    use archiver::web::models::NewUser;
    use archiver::web::models::Session;
    use archiver::web::models::User;

    use archiver::web::oauth::Oauth2Config;

    fn client() -> Client {
        Client::new(super::configure_rocket(true)).expect("valid rocket instance")
    }

    fn db_conn(client: &Client) -> DbConn {
        DbConn::maybe_from_rocket(client.rocket()).expect("db connection")
    }

    fn get_set_cookie(response: LocalResponse<'_>, name: &str) -> Option<String> {
        for cookie in response.headers().get("Set-Cookie") {
            if cookie.starts_with(&format!("{}=", name)) {
                return Some(cookie.to_owned());
            }
        }

        None
    }

    fn signin(client: &Client, username: &str, password: &str) -> Option<String> {
        let req = client
            .post("/signin")
            .header(ContentType::Form)
            .body(format!(
                "email={}&password={}&action=signin",
                username, password
            ));

        let response = req.dispatch();
        return get_set_cookie(response, "sid");
    }

    fn create_user(client: &Client, username: &str, password: &str) -> User {
        let conn = db_conn(&client);

        NewUser::new(username, password).create(&*conn).unwrap()
    }

    fn init_env() {
        env::set_var("ARCHIVER_BASE_URL", "http://localhost:8000/");
        env::set_var("ARCHIVER_DROPBOX_APP_KEY", "app_key");
        env::set_var("ARCHIVER_DROPBOX_APP_SECRET", "secret_key");
    }

    fn session_from_cookie(client: &Client, session_cookie: String) -> Option<Session> {
        let conn = db_conn(&client);
        let rest = session_cookie.split("sid=").nth(1)?;
        let session_id = rest.split(";").nth(0)?;
        Session::by_id(&*conn, session_id)
            .ok()
            .map(|(session, _)| session)
    }

    #[test]
    fn test_signin() {
        let client = client();

        create_user(&client, "test@email.com", "p@55w0rd");

        let req = client
            .post("/signin")
            .header(ContentType::Form)
            .body(r"email=test%40email.com&password=p%4055w0rd&action=signin");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/"));
        assert!(get_set_cookie(response, "sid").is_some())
    }

    #[test]
    fn test_signup() {
        let client = client();
        let req = client
            .post("/signin")
            .header(ContentType::Form)
            .body(r"email=test%40email.com&password=p%4055w0rd&action=signup");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/"));
        assert!(get_set_cookie(response, "sid").is_some())
    }

    #[test]
    fn test_failed_signin() {
        let client = client();
        let req = client
            .post("/signin")
            .header(ContentType::Form)
            .body(r"email=test%40email.com&password=p%4055w0rd&action=signin");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/signin"));
        assert_eq!(
            get_set_cookie(response, "_flash").unwrap(),
            "_flash=5errorIncorrect%20username%20or%20password.; Path=/; Max-Age=300"
        )
    }

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
        let config =
            Config::from_str(&response.body_string().expect("Didn't recieve a body")).unwrap();
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
        let config =
            Config::from_str(&response.body_string().expect("Didn't recieve a body")).unwrap();
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
    fn test_revoke_api_token() {
        let client = client();

        let user = create_user(&client, "test@email.com", "p@55w0rd");

        let token = {
            let conn = db_conn(&client);

            NewKey::new(&user).create(&*conn).unwrap()
        };

        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();
        let req = client
            .post("/key/expire")
            .header(ContentType::Form)
            .body(&format!("key_id={}", token.id));

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        let conn = db_conn(&client);
        assert!(
            user.key_by_id(token.id, &*conn).unwrap().expired.is_some(),
            "Didn't expire token"
        );
    }

    #[test]
    fn test_cannot_revoke_other_users_api_token() {
        let client = client();

        let user1 = create_user(&client, "ohno", "badpw");
        let _user2 = create_user(&client, "lolwat", "worse");

        let token = {
            let conn = db_conn(&client);

            NewKey::new(&user1).create(&*conn).unwrap()
        };

        signin(&client, "lolwat", "worse").unwrap();
        let req = client
            .post("/key/expire")
            .header(ContentType::Form)
            .body(&format!("key_id={}", token.id));

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        let conn = db_conn(&client);
        assert!(
            user1.key_by_id(token.id, &*conn).unwrap().expired.is_none(),
            "Expired another user's token"
        );
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
        let config =
            Config::from_str(&response.body_string().expect("Didn't recieve a body")).unwrap();
        let backend_names: Vec<_> = config.backends().iter().map(|b| b.name()).collect();
        assert_eq!(&backend_names, &["dropbox"]);

        let empty_flysights: Vec<FlysightConfig> = vec![];
        assert_eq!(config.flysights(), &empty_flysights);
        assert_eq!(config.mass_storages().len(), 1);
        assert_eq!(config.gopros().len(), 1);
    }

    #[test]
    fn test_signout() {
        let client = client();

        create_user(&client, "test@email.com", "p@55w0rd");
        let session_cookie = signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let req = client.post("/signout");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert!(get_set_cookie(response, "sid")
            .unwrap()
            .starts_with("sid=;"));

        assert_eq!(session_from_cookie(&client, session_cookie), None);
    }

    #[test]
    fn test_connect_integration() {
        init_env();

        let client = client();
        create_user(&client, "test@email.com", "p@55w0rd");
        let session_cookie = signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let req = client
            .post("/integration")
            .header(ContentType::Form)
            .body(r"provider=dropbox");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        assert!(response
            .headers()
            .get_one("Location")
            .unwrap()
            .starts_with(Oauth2Config::dropbox().auth_url.as_str()));

        let session = session_from_cookie(&client, session_cookie).unwrap();
        assert!(session.data.get("dropbox").unwrap().is_string());
    }

    #[test]
    fn test_disconnect_integration() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let integration_id = {
            let conn = db_conn(&client);

            NewIntegration::new(&user, "dropbox", "test_oauth_token")
                .create(&*conn)
                .unwrap()
                .id
        };

        let req = client
            .post("/integration/disconnect")
            .header(ContentType::Form)
            .body(format!(
                "provider=dropbox&integration_id={}",
                integration_id
            ));

        let response = req.dispatch();

        assert_eq!(response.status(), Status::SeeOther);

        let conn = db_conn(&client);
        assert_eq!(user.integrations(&*conn).unwrap().len(), 0);
    }

    #[test]
    fn test_finish_integration() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        let session_cookie = signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let mut session = session_from_cookie(&client, session_cookie.clone()).unwrap();

        {
            let conn = db_conn(&client);
            session.insert("dropbox".to_string(), "test_csrf_token".into());
            session.save(&*conn).unwrap();
        }

        let req = client
            .get("/integration/finish?provider=dropbox&state=test_csrf_token&code=test_oauth_token")
            .header(Header::new("Cookie", session_cookie.clone()));

        let response = req.dispatch();

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/"));

        let conn = db_conn(&client);

        assert_eq!(
            user.integrations(&*conn)
                .unwrap()
                .first()
                .unwrap()
                .access_token,
            "test_oauth_token"
        );
    }

    #[test]
    fn test_connect_integration_doesnt_stomp_on_sessions() {
        init_env();

        let client1 = client();
        let client2 = client();
        let _u1 = create_user(&client1, "test1@email.com", "p@55w0rd");
        let _u2 = create_user(&client2, "test2@email.com", "p@55w0rd");

        let s1 = signin(&client1, "test1%40email.com", "p%4055w0rd").unwrap();
        let s2 = signin(&client2, "test2%40email.com", "p%4055w0rd").unwrap();

        let session1 = session_from_cookie(&client1, s1.clone()).unwrap();
        let session2 = session_from_cookie(&client2, s2.clone()).unwrap();

        assert!(
            session1.user_id != session2.user_id,
            "User IDs have been tampered with"
        );

        let req = client1
            .post("/integration")
            .header(ContentType::Form)
            .body(r"provider=dropbox");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        assert!(response
            .headers()
            .get_one("Location")
            .unwrap()
            .starts_with(Oauth2Config::dropbox().auth_url.as_str()));

        let session1 = session_from_cookie(&client1, s1.clone()).unwrap();
        let session2 = session_from_cookie(&client2, s2.clone()).unwrap();

        assert!(session1.data.get("dropbox").unwrap().is_string());
        assert!(session2.data.get("dropbox").is_none());
        assert!(
            session1.user_id != session2.user_id,
            "User IDs have been tampered with"
        );
    }

    #[test]
    fn test_create_devices() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        for (name, kind, identifier) in &[
            ("gopro5", "ptp", "C123456"),
            ("comp", "flysight", "/mnt/flysight"),
            ("sdcard", "mass_storage", "/media/sdcard"),
        ] {
            let req = client
                .post("/device")
                .header(ContentType::Form)
                .body(format!(
                    "name={}&kind={}&identifier={}",
                    name, kind, identifier
                ));

            let response = req.dispatch();

            assert_eq!(response.status(), Status::SeeOther);
            assert_eq!(response.headers().get_one("Location"), Some("/"));
        }

        let conn = db_conn(&client);

        let devices = user.devices(&*conn).unwrap();
        assert_eq!(devices.len(), 3);
    }

    #[test]
    fn test_invalid_device_type() {
        init_env();

        let client = client();
        create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let add_device = |kind, name| {
            let req = client
                .post("/device")
                .header(ContentType::Form)
                .body(format!("kind={}&identifier={}", &kind, &name));

            let response = req.dispatch();

            assert_eq!(response.status(), Status::UnprocessableEntity);
        };

        add_device("nonexistant", "gopro5");
    }

    #[test]
    fn test_delete_devices() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let device_id = {
            let conn = db_conn(&client);

            NewDevice::new(&user, "gopro", "ptp", "test_gopro")
                .create(&*conn)
                .unwrap()
                .id
        };

        {
            let conn = db_conn(&client);

            assert_eq!(user.devices(&*conn).unwrap().len(), 1);
        }

        let req = client
            .post("/device/delete")
            .header(ContentType::Form)
            .body(format!("kind=ptp&device_id={}", device_id));

        let response = req.dispatch();

        assert_eq!(response.status(), Status::SeeOther);

        let conn = db_conn(&client);
        assert_eq!(user.devices(&*conn).unwrap().len(), 0);
    }

    #[test]
    fn test_json_signin() {
        let client = client();

        let _user = create_user(&client, "test@email.com", "p@55w0rd");

        let req = client
            .post("/json/signin")
            .header(ContentType::JSON)
            .body("{\"email\": \"test@email.com\", \"password\": \"p@55w0rd\"}");

        let mut response = req.dispatch();
        assert_eq!(response.status(), Status::Ok);
        let message =
            serde_json::from_str::<messages::JsonSignInResp>(&response.body_string().unwrap())
                .unwrap();
        assert!(
            match message {
                messages::JsonSignInResp::Token(_) => true,
                messages::JsonSignInResp::Error(_) => false,
            },
            "Didn't get a token"
        );
    }

    #[test]
    fn test_json_signin_invalid_credentials() {
        let client = client();

        let _user = create_user(&client, "test@email.com", "p@55w0rd");

        let req = client
            .post("/json/signin")
            .header(ContentType::JSON)
            .body("{\"email\": \"test@email.com\", \"password\": \"buttsbutts\"}");

        let mut response = req.dispatch();
        assert_eq!(response.status(), Status::Ok);
        let message =
            serde_json::from_str::<messages::JsonSignInResp>(&response.body_string().unwrap())
                .unwrap();
        assert!(
            match message {
                messages::JsonSignInResp::Token(_) => false,
                messages::JsonSignInResp::Error(_) => true,
            },
            "Didn't get an error"
        );
    }
}
