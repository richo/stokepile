use std::env;

use rocket::http::{ContentType};
use rocket::local::{Client, LocalResponse};

use crate::web::db::DbConn;
use crate::web::models::NewUser;
use crate::web::models::Session;
use crate::web::models::User;

use crate::messages;

use crate::config::{MountableDeviceLocation, StagingConfig};

pub fn db_conn(client: &Client) -> DbConn {
    DbConn::maybe_from_rocket(client.rocket()).expect("db connection")
}

pub fn get_set_cookie(response: &LocalResponse<'_>, name: &str) -> Option<String> {
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
    return get_set_cookie(&response, "sid");
}

pub fn signin_api(client: &Client, username: &str, password: &str) -> Option<String> {
    let json = serde_json::to_string(&messages::JsonSignIn {
        email: username.into(),
        password: password.into(),
    }).expect("couldn't serialize login info");
    let req = client
        .post("/json/signin")
        .header(ContentType::JSON)
        .body(json);

    let mut response = req.dispatch();
    let body: messages::JsonSignInResp =
        serde_json::from_str(&response.body_string().expect("Didn't recieve a body")).unwrap();
    body.into_result().ok()
}

pub fn create_user(client: &Client, username: &str, password: &str) -> User {
    let conn = db_conn(&client);

    let user = NewUser::new(username, password).create(&*conn).unwrap();

    user.update_staging(&StagingConfig {
        location: MountableDeviceLocation::Mountpoint("/path".into())
    }, &*conn).unwrap();

    user
}

pub fn create_admin(client: &Client, username: &str, password: &str) -> User {
    let user = create_user(client, username, password);
    let conn = db_conn(&client);

    {
        use diesel::update;
        use crate::web::schema::users::dsl::*;
        use diesel::prelude::*;

        update(&user)
            .set(admin.eq(true))
            .get_result(&*conn).expect("Couldn't promote user to Admin")
    }
}

pub fn disable_signups_without_invites(client: &Client) {
    use diesel::update;
    use crate::web::schema::global_settings::dsl::*;
    use diesel::prelude::*;

    let conn = db_conn(&client);

    update(global_settings)
        .set(invites_required.eq(true))
        .execute(&*conn).expect("Couldn't disable signups without invites");
}

pub fn enable_signups_without_invites(client: &Client) {
    use diesel::update;
    use crate::web::schema::global_settings::dsl::*;
    use diesel::prelude::*;

    let conn = db_conn(&client);

    update(global_settings)
        .set(invites_required.eq(false))
        .execute(&*conn).expect("Couldn't disable signups without invites");
}

pub fn init_env() {
    env::set_var("STOKEPILE_BASE_URL", "http://localhost:8000/");
    env::set_var("STOKEPILE_DROPBOX_APP_KEY", "app_key");
    env::set_var("STOKEPILE_DROPBOX_APP_SECRET", "secret_key");
    env::set_var("STOKEPILE_GOOGLE_APP_KEY", "app_key");
    env::set_var("STOKEPILE_GOOGLE_APP_SECRET", "secret_key");
}

pub fn session_from_cookie(client: &Client, session_cookie: String) -> Option<Session> {
    let conn = db_conn(&client);
    let rest = session_cookie.split("sid=").nth(1)?;
    let session_id = rest.split(";").nth(0)?;
    Session::by_id(&*conn, session_id)
        .ok()
        .map(|(session, _)| session)
}

pub mod rigging {
    use diesel::prelude::*;
    use crate::web::models::{
        Customer, NewCustomer,
        User
    };

    pub fn create_customer(user: &User, conn: &PgConnection) -> Customer {
        // TODO(richo) Random details here?
        NewCustomer {
            user_id: user.id,
            name: "name",
            address: "address",
            phone_number: "phone",
            email: "email",
        }.create(conn).expect("Couldn't create test customer")
    }
}
