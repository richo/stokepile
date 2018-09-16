use rocket::Outcome;
use rocket::http::Status;
use rocket::request::{self, Request, FromRequest};

pub mod nav;
pub mod oauth2;

use web::nav::{NavMap, NavEntry};

#[derive(Serialize)]
pub struct Ctx {
    nav: NavMap,
}

impl<'a, 'r> FromRequest<'a, 'r> for Ctx {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Ctx, ()> {
        let mut ctx = Ctx {
            nav: nav::NAV_MAP.clone(),
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

