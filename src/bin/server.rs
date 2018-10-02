#![feature(plugin, custom_derive, decl_macro, proc_macro_non_items)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate log;

extern crate diesel;
extern crate failure;
extern crate pretty_env_logger;

#[macro_use]
extern crate rocket;
extern crate rocket_contrib;

extern crate archiver;

use failure::Error;
use rocket::http::RawStr;
use rocket::http::{Cookie, Cookies, SameSite};
use rocket::request::{FlashMessage, Form, FromFormValue};
use rocket::response::{Flash, Redirect};
use rocket::Rocket;
use rocket_contrib::static_files::StaticFiles;
use rocket_contrib::{Json, Template};

use std::env;

use archiver::config::Config;
use archiver::web::auth::CurrentUser;
use archiver::web::context::Context;
use archiver::web::db::{init_pool, DbConn};
use archiver::web::models::{NewSession, NewUser, User};

#[get("/config")]
fn get_config(_user: CurrentUser) -> Result<Json<Config>, Error> {
    let config = Config::from_file("archiver.toml.example")?;
    info!("Butts");
    Ok(Json(config))
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
                    // TODO: make this cookie secure depending on env
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
fn signout(mut cookies: Cookies) -> Redirect {
    // TODO: Mark session as expired in DB.
    cookies.remove(Cookie::named("sid"));
    Redirect::to("/")
}

#[get("/")]
fn index(user: Option<CurrentUser>) -> Template {
    let context = Context::default().set_user(user);
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
        .mount("/", routes![get_config, get_signin, signin, signout, index])
        .mount("/static", StaticFiles::from("web/static"))
        .attach(Template::fairing())
}

fn main() {
    init_logging();
    configure_rocket(false).launch();
}

#[cfg(test)]
mod tests {
    use rocket::http::{ContentType, Header, Status};
    use rocket::local::{Client, LocalResponse};

    use archiver::web::db::DbConn;
    use archiver::web::models::NewUser;

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

    fn create_user(client: &Client, username: &str, password: &str) {
        let conn = db_conn(&client);

        NewUser::new(username, password).create(&*conn).unwrap();
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
    fn test_get_config() {
        let client = client();

        create_user(&client, "test@email.com", "p@55w0rd");
        let session_cookie = signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let req = client
            .get("/config")
            .header(Header::new("Cookie", session_cookie));

        let response = req.dispatch();
        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn test_signout() {
        let client = client();

        create_user(&client, "test@email.com", "p@55w0rd");
        let session_cookie = signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let req = client
            .post("/signout")
            .header(Header::new("Cookie", session_cookie));

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert!(
            get_set_cookie(response, "sid")
                .unwrap()
                .starts_with("sid=;")
        );
    }
}
