use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::Outcome;

use crate::web::db::DbConn;
use crate::web::models::{Key, Session, User};

#[derive(Debug, Serialize)]
pub struct WebUser {
    #[serde(flatten)]
    pub user: User,
    #[serde(skip_serializing)]
    pub session: Session,
}

#[derive(Debug, Serialize)]
pub struct AdminUser {
    #[serde(flatten)]
    pub user: User,
    #[serde(skip_serializing)]
    pub session: Session,
}

impl WebUser {
    pub fn new(user: User, session: Session) -> Self {
        WebUser { user, session }
    }

    // TODO(richo) move this into a trait?
    pub fn id(&self) -> i32 {
        self.user.id
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for WebUser {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let conn = request.guard::<DbConn>()?;

        let current_user = {
            if let Some(cookie) = request.cookies().get("sid") {
                match Session::by_id(&*conn, cookie.value()) {
                    Ok((session, user)) => Some(WebUser::new(user, session)),
                    Err(_) => None,
                }
            } else {
                None
            }
        };
        current_user.map_or(
            Outcome::Failure((Status::Unauthorized, ())),
            Outcome::Success,
        )
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for AdminUser {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let user = WebUser::from_request(request)?;
        if user.user.is_admin() {
            // Transmute the User into an Admin
            let WebUser { user, session } = user;
            return Outcome::Success(AdminUser { user, session });
        }

        Outcome::Failure((Status::Unauthorized, ()))
    }
}

impl From<AdminUser> for WebUser {
    fn from(admin: AdminUser) -> Self {
        let AdminUser { user, session } = admin;
        return WebUser { user, session };
    }
}

#[derive(Debug, Serialize)]
pub struct ApiUser {
    #[serde(flatten)]
    pub user: User,
    #[serde(skip_serializing)]
    pub key: Key,
}

impl ApiUser {
    pub fn new(user: User, key: Key) -> Self {
        ApiUser { user, key }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for ApiUser {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let conn = request.guard::<DbConn>()?;

        let current_user = {
            if let Some(token) = request.headers().get_one("authorization") {
                let api_token = token.trim_start_matches("Bearer: ");
                match Key::by_token(&*conn, api_token) {
                    Ok((key, user)) => {
                        if key.is_expired() {
                            None
                        } else {
                            Some(ApiUser::new(user, key))
                        }
                    }
                    Err(_) => None,
                }
            } else {
                None
            }
        };
        current_user.map_or(
            Outcome::Failure((Status::Unauthorized, ())),
            Outcome::Success,
        )
    }
}

#[derive(Debug)]
pub enum AuthenticatedUser {
    Web(WebUser),
    Api(ApiUser),
}

impl AuthenticatedUser {
    pub fn user(&self) -> &User {
        match self {
            AuthenticatedUser::Web(web) => &web.user,
            AuthenticatedUser::Api(api) => &api.user,
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for AuthenticatedUser {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        if let Outcome::Success(web) = WebUser::from_request(request) {
            return Outcome::Success(AuthenticatedUser::Web(web));
        }
        if let Outcome::Success(api) = ApiUser::from_request(request) {
            return Outcome::Success(AuthenticatedUser::Api(api));
        }
        return Outcome::Failure((Status::Unauthorized, ()));
    }
}
