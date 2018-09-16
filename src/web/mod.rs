use rocket::Outcome;
use rocket::http::Status;
use rocket::request::{self, Request, FromRequest};

pub mod nav;
pub mod oauth2;

use web::nav::{NavMap, NavEntry};

#[derive(Serialize)]
pub struct Ctx<T> {
    nav: NavMap,
    pub local: Option<T>,
}

impl<'a, 'r, T> FromRequest<'a, 'r> for Ctx<T> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Ctx<T>, ()> {
        let mut ctx = Ctx {
            nav: nav::NAV_MAP.clone(),
            local: None,
        };

        let mut found = false;
        for mut i in ctx.nav.routes.iter_mut() {
            if i.location == request.uri().path() {
                i.active = true;
            }
        }
        return Outcome::Success(ctx);
    }
}

