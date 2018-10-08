#![feature(plugin, custom_derive, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate log;

extern crate diesel;
extern crate failure;
extern crate pretty_env_logger;

extern crate dotenv;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate rocket;
extern crate rocket_contrib;

extern crate oauth2;

extern crate archiver;

use rocket::config::Environment;
use rocket::response::content::Content;
use rocket::http::ContentType;
use rocket::http::RawStr;
use rocket::http::{Cookie, Cookies, SameSite};
use rocket::request::{FlashMessage, Form, FromFormValue};
use rocket::response::{Flash, Redirect};
use rocket::Rocket;
use rocket_contrib::static_files::StaticFiles;
use rocket_contrib::Template;

use std::env;

use oauth2::prelude::*;
use oauth2::CsrfToken;

use archiver::config::Config;
use archiver::web::auth::CurrentUser;
use archiver::web::context::{Context, PossibleIntegration};
use archiver::web::db::{init_pool, DbConn};
use archiver::web::models::{Integration, NewIntegration, NewSession, NewUser, User};
use archiver::web::oauth::Oauth2Provider;

lazy_static! {
    static ref ROCKET_ENV: Environment = Environment::active().expect("Could not get ROCKET_ENV.");
}

#[get("/config")]
fn get_config(user: CurrentUser, conn: DbConn) -> Result<Content<String>, Flash<Redirect>> {
    let mut vimeo = None;
    let mut dropbox = None;

    let integrations = user.user.integrations(&*conn).map_err(|e| Flash::error(
            Redirect::to("/"),
            format!("Error connecting to the DB: {}", e)))?;
    let mut integrations = integrations.iter();
    for provider in Oauth2Provider::providers() {
        let name = provider.name();

        if let Some(integration) = integrations.find(|ref x| x.provider == name) {
            let token = integration.access_token.to_string();
            match name {
                "dropbox" => { dropbox = Some(token); },
                "vimeo" => { vimeo = Some(token); },
                name => { warn!("Unknown integration: {}", name); },
            }
        }
    }
    match Config::from_db(dropbox, vimeo) {
        Ok(config) => Ok(Content(ContentType::new("application", "toml"), config.to_toml())),
        Err(error) => Err(Flash::error(
            Redirect::to("/"),
            format!(
                "There was a problem generating configuration for you: {}",
                error
            ),
        ))
    }
}

enum UserAction {
    SignIn,
    SignUp,
}

impl<'v> FromFormValue<'v> for UserAction {
    type Error = String;

    fn from_form_value(form_value: &'v RawStr) -> Result<UserAction, Self::Error> {
        let decoded = form_value.url_decode();
        match decoded {
            Ok(ref action) if action == "signin" => Ok(UserAction::SignIn),
            Ok(ref action) if action == "signup" => Ok(UserAction::SignUp),
            _ => Err(format!("expected signin/signup not {}", form_value)),
        }
    }
}

#[derive(FromForm)]
struct SignInUpForm {
    email: String,
    password: String,
    action: UserAction,
}

// TODO: CSRF.
#[post("/signin", data = "<signin>")]
fn signin(
    conn: DbConn,
    signin: Form<SignInUpForm>,
    mut cookies: Cookies,
) -> Result<Redirect, Flash<Redirect>> {
    let user: Result<User, &str> = match signin.action {
        UserAction::SignIn => User::by_credentials(&*conn, &signin.email, &signin.password)
            .ok_or("Incorrect username or password."),
        UserAction::SignUp => NewUser::new(&signin.email, &signin.password)
            .create(&*conn)
            .map_err(|_| "Unable to signup"),
    };

    match user {
        Ok(user) => {
            let session = NewSession::new(&user).create(&*conn).unwrap();
            cookies.add(
                Cookie::build("sid", session.id)
                    .secure(!ROCKET_ENV.is_dev())
                    .http_only(true)
                    .same_site(SameSite::Lax)
                    .finish(),
            );
            Ok(Redirect::to("/"))
        }
        Err(message) => Err(Flash::error(Redirect::to("/signin"), message)),
    }
}

#[get("/signin")]
fn get_signin<'r>(flash: Option<FlashMessage>) -> Template {
    let context = Context::default().set_signin_error(flash.map(|msg| msg.msg().into()));
    Template::render("signin", context)
}

#[post("/signout")]
fn signout(user: CurrentUser, conn: DbConn, mut cookies: Cookies) -> Redirect {
    user.session
        .delete(&*conn)
        .expect("Could not delete session.");
    cookies.remove(Cookie::named("sid"));
    Redirect::to("/")
}

#[derive(FromForm)]
struct DisconnectForm {
    integration_id: i32,
    provider: Oauth2Provider,
}

#[post("/integration/disconnect", data = "<disconnect>")]
fn disconnect_integration(
    user: CurrentUser,
    disconnect: Form<DisconnectForm>,
    conn: DbConn,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    user.user
        .integration_by_id(disconnect.integration_id, &*conn)
        .map(|i| i.delete(&*conn))
        .map(|_| {
            Flash::success(
                Redirect::to("/"),
                format!(
                    "{} has been disconnected from your account.",
                    disconnect.provider.display_name()
                ),
            )
        }).map_err(|e| {
            warn!("{}", e);
            Flash::error(
                Redirect::to("/"),
                format!(
                    "{} could not be disconnected from your account.",
                    disconnect.provider.display_name()
                ),
            )
        })
}

#[derive(FromForm)]
struct ConnectForm {
    provider: Oauth2Provider,
}

#[post("/integration", data = "<connect>")]
fn connect_integration(
    mut user: CurrentUser,
    conn: DbConn,
    connect: Form<ConnectForm>,
) -> Redirect {
    let client = connect.provider.client();

    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random);

    user.session.insert(
        connect.provider.name().into(),
        csrf_state.secret().to_string().into(),
    );

    user.session.save(&*conn).unwrap();

    Redirect::to(authorize_url.as_str().to_string())
}

#[derive(FromForm, Debug)]
pub struct Oauth2Response {
    pub provider: Oauth2Provider,
    pub state: String,
    pub code: String,
    pub scope: Option<String>,
}

#[get("/integration/finish?<resp>")]
fn finish_integration(
    user: CurrentUser,
    resp: Oauth2Response,
    conn: DbConn,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let integration: Option<Integration> = if user
        .session
        .data
        .get(resp.provider.name())
        .map(|state| state.as_str())
        != Some(Some(&resp.state))
    {
        warn!(
            "user {:?} oauth state didn't match for provider: {:?}",
            user.user.id, resp.provider
        );
        None
    } else {
        NewIntegration::new(&user.user, resp.provider.name(), &resp.code)
            .create(&*conn)
            .ok()
    };

    match integration {
        Some(_) => Ok(Flash::success(
            Redirect::to("/"),
            format!(
                "{} has been connected to your account.",
                resp.provider.display_name()
            ),
        )),
        None => Err(Flash::error(
            Redirect::to("/"),
            format!(
                "There was a problem connecting {} to your account.",
                resp.provider.display_name()
            ),
        )),
    }
}

#[get("/")]
fn index(user: Option<CurrentUser>, conn: DbConn, flash: Option<FlashMessage>) -> Template {
    let mut possible_integrations = vec![];

    if let Some(user) = &user {
        if let Ok(integrations) = user.user.integrations(&*conn) {
            let mut integrations = integrations.iter();

            for provider in Oauth2Provider::providers() {
                let name = provider.name();

                let configured_integration = integrations.find(|ref x| x.provider == name);

                possible_integrations.push(PossibleIntegration {
                    id: configured_integration.map(|i| i.id),
                    name: provider.name(),
                    display_name: provider.display_name(),
                    connected: configured_integration.is_some(),
                });
            }
        }
    }

    let context = Context::default()
        .set_user(user)
        .set_integrations(possible_integrations)
        .set_integration_message(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("index", context)
}

fn init_logging() {
    if let None = env::var_os("RUST_LOG") {
        env::set_var("RUST_LOG", "INFO");
    }
    pretty_env_logger::init();
}

fn configure_rocket(test_transactions: bool) -> Rocket {
    rocket::ignite()
        .manage(init_pool(test_transactions))
        .mount(
            "/",
            routes![
                get_config,
                get_signin,
                signin,
                signout,
                index,
                connect_integration,
                disconnect_integration,
                finish_integration
            ],
        ).mount("/static", StaticFiles::from("web/static"))
        .attach(Template::fairing())
}

fn main() {
    dotenv::dotenv().ok();
    init_logging();
    configure_rocket(false).launch();
}

#[cfg(test)]
mod tests {
    use std::env;

    use rocket::http::{ContentType, Header, Status};
    use rocket::local::{Client, LocalResponse};

    use archiver::config::Config;

    use archiver::web::db::DbConn;
    use archiver::web::models::NewIntegration;
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

    fn get_set_cookie(response: LocalResponse, name: &str) -> Option<String> {
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

        let integration_id = {
            let conn = db_conn(&client);

            NewIntegration::new(&user, "dropbox", "test_oauth_token")
                .create(&*conn)
                .unwrap()
                .id
        };

        let req = client.get("/config");

        let mut response = req.dispatch();
        assert_eq!(response.status(), Status::Ok);
        let config = Config::from_str(&response.body_string().expect("Didn't recieve a body")).unwrap();
        let backend_names: Vec<_> = config.backends().iter().map(|b| b.name()).collect();
        assert_eq!(&backend_names, &["dropbox"]);
    }

    #[test]
    fn test_signout() {
        let client = client();

        create_user(&client, "test@email.com", "p@55w0rd");
        let session_cookie = signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let req = client.post("/signout");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert!(
            get_set_cookie(response, "sid")
                .unwrap()
                .starts_with("sid=;")
        );

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

        assert!(
            response
                .headers()
                .get_one("Location")
                .unwrap()
                .starts_with(Oauth2Config::dropbox().auth_url.as_str())
        );

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

        assert!(
            response
                .headers()
                .get_one("Location")
                .unwrap()
                .starts_with(Oauth2Config::dropbox().auth_url.as_str())
        );

        let session1 = session_from_cookie(&client1, s1.clone()).unwrap();
        let session2 = session_from_cookie(&client2, s2.clone()).unwrap();

        assert!(session1.data.get("dropbox").unwrap().is_string());
        assert!(session2.data.get("dropbox").is_none());
        assert!(
            session1.user_id != session2.user_id,
            "User IDs have been tampered with"
        );
    }

}
