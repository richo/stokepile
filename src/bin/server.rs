#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate log;

extern crate diesel;
extern crate failure;
extern crate pretty_env_logger;
extern crate rocket;
extern crate rocket_contrib;

extern crate archiver;

use failure::Error;
use rocket::request::Form;
use rocket::response::Redirect;
use rocket::Rocket;
use rocket_contrib::Json;
use std::env;

use archiver::config::Config;
use archiver::web::db::{init_pool, DbConn};
use archiver::web::models::{NewUser, User};

#[get("/config")]
fn get_config() -> Result<Json<Config>, Error> {
    let config = Config::from_file("archiver.toml.example")?;
    info!("Butts");
    Ok(Json(config))
}

#[derive(FromForm)]
struct SignUp {
    email: String,
    password: String,
}

#[post("/signup", data = "<signup_form>")]
fn signup(conn: DbConn, signup_form: Form<SignUp>) -> Redirect {
    let signup = signup_form.get();

    NewUser::new(&signup.email, &signup.password)
        .create(&*conn)
        .unwrap();

    Redirect::to("/")
}

#[derive(FromForm)]
struct SignIn {
    email: String,
    password: String,
}

#[post("/signin", data = "<signin_form>")]
fn signin(conn: DbConn, signin_form: Form<SignIn>) -> Redirect {
    let signin = signin_form.get();

    match User::by_credentials(&*conn, &signin.email, &signin.password) {
        Some(_user) => Redirect::to("/"),
        None => Redirect::to("/error"),
    }
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
        .mount("/", routes![get_config, signup, signin])
}

fn main() {
    init_logging();
    configure_rocket(false).launch();
}

#[cfg(test)]
mod tests {
    use rocket::http::{ContentType, Status};
    use rocket::local::Client;

    use archiver::web::db::DbConn;
    use archiver::web::models::NewUser;

    fn client() -> Client {
        Client::new(super::configure_rocket(true)).expect("valid rocket instance")
    }

    fn db_conn(client: &Client) -> DbConn {
        DbConn::maybe_from_rocket(client.rocket()).expect("db connection")
    }

    #[test]
    fn test_signin() {
        let client = client();

        {
            let conn = db_conn(&client);

            NewUser::new("test@email.com", "p@55w0rd")
                .create(&*conn)
                .unwrap();
        }

        let req = client
            .post("/signin")
            .header(ContentType::Form)
            .body(r"email=test%40email.com&password=p%4055w0rd");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/"));
    }

    #[test]
    fn test_signup() {
        let client = client();
        let req = client
            .post("/signup")
            .header(ContentType::Form)
            .body(r"email=test%40email.com&password=p%4055w0rd");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/"));
    }
}
