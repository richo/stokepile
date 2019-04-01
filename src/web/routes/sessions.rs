use crate::web::db::DbConn;
use crate::web::auth::{WebUser, ApiUser};
use crate::web::ROCKET_ENV;
use crate::web::context::Context;
use crate::messages::{self, Oauth2Provider, RefreshToken};
use oauth2::prelude::*;
use oauth2::TokenResponse;

use rocket::http::RawStr;
use rocket::http::{Cookie, Cookies, SameSite};
use rocket::request::{FlashMessage, Form, FromFormValue};
use rocket::response::{Flash, Redirect};
use rocket_contrib::json::Json;
use rocket_contrib::templates::Template;

use crate::web::models::{
    NewKey, NewSession, NewUser, User,
};

#[derive(FromForm, RedactedDebug)]
pub struct SignInUpForm {
    email: String,
    #[redacted]
    password: String,
    action: UserAction,
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
pub fn post_signin(
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

/// Allows an authenticated user to fetch a refresh token for a given service, which will then
/// allow them to interact with the given service.
///
/// This is mostly for google properties which don't support long lived API keys.
#[get("/refresh_token/<provider>")]
pub fn refresh_token(
    conn: DbConn,
    provider: Oauth2Provider,
    user: ApiUser,
) -> Result<Json<RefreshToken>, Json<RefreshToken>> {
    let client = provider.client();

    let integrations = user.user.integrations(&*conn).map_err(|e| {
        Json(RefreshToken::Error(e.to_string()))
    })?;

    if let Some(integration) = integrations.iter().find(|ref x| x.provider == provider.name()) {
        let refresh_token = integration.refresh_token()
            .ok_or(Json(RefreshToken::NotConfigured))?;
        // TODO(richo) definitely take this logic and put it elsewhere
        match client.exchange_refresh_token(&refresh_token) {
            // TODO(richo) Store the updated stuff
            // Do we need to store the new refresh token somewhere?
            // We also definitely do need to cache this token since goog apparantly don't like being
            // pummeled
            Ok(token) => Ok(Json(RefreshToken::Token(token.access_token().secret().to_owned()))),
            Err(error) => Ok(Json(RefreshToken::Error(error.to_string()))),
        }
    } else {
        Ok(Json(RefreshToken::NotConfigured))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;

    use rocket::http::{ContentType, Status};

    client_for_routes!(get_signin, signout, expire_key => client);

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

    #[test]
    fn test_google_credentials_refresh() {
        panic!()
    }

    #[test]
    fn test_integrations_not_configured_dtrt() {
        panic!()
    }

    #[test]
    fn test_unrefreshable_credentials_just_return_the_token() {
        panic!()
    }
}
