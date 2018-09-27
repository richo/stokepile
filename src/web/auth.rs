use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::Outcome;

use web::db::DbConn;
use web::models::Session;

pub struct CurrentUser {
    session: Session,
}

impl CurrentUser {
    pub fn from_session(session: Session) -> Self {
        CurrentUser { session: session }
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for CurrentUser {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let conn = request.guard::<DbConn>()?;

        let current_user = {
            if let Some(cookie) = request.cookies().get("sid") {
                match Session::by_id(&*conn, cookie.value()) {
                    Ok(session) => Some(CurrentUser::from_session(session)),
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
