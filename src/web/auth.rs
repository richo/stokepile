use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::Outcome;

use web::db::DbConn;
use web::models::{Session, User};

#[derive(Debug, Serialize)]
pub struct CurrentUser {
    #[serde(flatten)]
    user: User,
    #[serde(skip_serializing)]
    session: Session,
}

impl CurrentUser {
    pub fn new(user: User, session: Session) -> Self {
        CurrentUser {
            user: user,
            session: session,
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for CurrentUser {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let conn = request.guard::<DbConn>()?;

        let current_user = {
            if let Some(cookie) = request.cookies().get("sid") {
                match Session::by_id(&*conn, cookie.value()) {
                    Ok((session, user)) => Some(CurrentUser::new(user, session)),
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
