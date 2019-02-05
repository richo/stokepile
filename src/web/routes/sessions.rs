use std::fmt;

use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::ROCKET_ENV;
use crate::web::context::Context;
use crate::messages;

use rocket::http::RawStr;
use rocket::http::{Cookie, Cookies, SameSite};
use rocket::request::{FlashMessage, Form, FromFormValue};
use rocket::response::{Flash, Redirect};
use rocket_contrib::json::Json;
use rocket_contrib::templates::Template;

use crate::web::models::{
    NewKey, NewSession, NewUser, User,
};

#[derive(FromForm)]
pub struct SignInUpForm {
    email: String,
    password: String,
    action: UserAction,
}

impl fmt::Debug for SignInUpForm {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("SignInUpForm")
            .field("email", &self.email)
            .field("password", &"...")
            .field("action", &self.action)
            .finish()
    }
}

#[derive(Debug)]
pub enum UserAction {
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

// TODO: CSRF.
#[post("/signin", data = "<signin>")]
pub fn signin(
    conn: DbConn,
    signin: Form<SignInUpForm>,
    mut cookies: Cookies<'_>,
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
pub fn get_signin<'r>(flash: Option<FlashMessage<'_, '_>>) -> Template {
    let context = Context::default().set_signin_error(flash.map(|msg| msg.msg().into()));
    Template::render("signin", context)
}

#[post("/signout")]
pub fn signout(user: WebUser, conn: DbConn, mut cookies: Cookies<'_>) -> Redirect {
    user.session
        .delete(&*conn)
        .expect("Could not delete session.");
    cookies.remove(Cookie::named("sid"));
    Redirect::to("/")
}

#[post("/json/signin", format = "json", data = "<signin>")]
pub fn signin_json(
    conn: DbConn,
    signin: Json<messages::JsonSignIn>,
) -> Json<messages::JsonSignInResp> {
    match User::by_credentials(&*conn, &signin.0.email, &signin.0.password) {
        Some(user) => {
            let key = NewKey::new(&user).create(&*conn).unwrap();
            Json(messages::JsonSignInResp::Token(key.token))
        }
        None => Json(messages::JsonSignInResp::Error(
            "Incorrect username or password.".to_string(),
        )),
    }
}

#[derive(Debug, FromForm)]
pub struct ExpireKeyForm {
    key_id: i32,
}

#[post("/key/expire", data = "<key>")]
pub fn expire_key(
    user: WebUser,
    conn: DbConn,
    key: Form<ExpireKeyForm>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    user.user
        .key_by_id(key.key_id, &*conn)
        .map(|i| i.expire(&*conn))
        .map(|_| Flash::success(Redirect::to("/"), format!("key has been expired.")))
        .map_err(|e| {
            warn!("{}", e);
            Flash::error(Redirect::to("/"), format!("the key could not be expired."))
        })
}
