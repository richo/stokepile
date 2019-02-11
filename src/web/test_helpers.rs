use std::env;

use rocket::http::{ContentType};
use rocket::local::{Client, LocalResponse};

use crate::web::db::DbConn;
use crate::web::models::NewUser;
use crate::web::models::Session;
use crate::web::models::User;
use crate::web::routes::settings::SettingsForm;

use crate::config::StagingConfig;

pub fn db_conn(client: &Client) -> DbConn {
    DbConn::maybe_from_rocket(client.rocket()).expect("db connection")
}

pub fn get_set_cookie(response: LocalResponse<'_>, name: &str) -> Option<String> {
    for cookie in response.headers().get("Set-Cookie") {
        if cookie.starts_with(&format!("{}=", name)) {
            return Some(cookie.to_owned());
        }
    }

    None
}

pub fn signin(client: &Client, username: &str, password: &str) -> Option<String> {
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

pub fn create_user(client: &Client, username: &str, password: &str) -> User {
    let conn = db_conn(&client);

    let user = NewUser::new(username, password).create(&*conn).unwrap();

    user.update_staging(&StagingConfig::StagingDirectory("/path".into()), &*conn).unwrap();

    user
}

pub fn init_env() {
    env::set_var("ARCHIVER_BASE_URL", "http://localhost:8000/");
    env::set_var("ARCHIVER_DROPBOX_APP_KEY", "app_key");
    env::set_var("ARCHIVER_DROPBOX_APP_SECRET", "secret_key");
}

pub fn session_from_cookie(client: &Client, session_cookie: String) -> Option<Session> {
    let conn = db_conn(&client);
    let rest = session_cookie.split("sid=").nth(1)?;
    let session_id = rest.split(";").nth(0)?;
    Session::by_id(&*conn, session_id)
        .ok()
        .map(|(session, _)| session)
}
