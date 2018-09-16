use rocket::Outcome;
use rocket::http::Status;
use rocket::request::{self, Request, FromRequest};

pub mod nav;
pub mod oauth2;

use web::nav::{NavMap, NavEntry};

#[derive(Serialize)]
pub struct Ctx<T> {
    nav: NavMap,
    integrations: Vec<Integration>,
    pub local: Option<T>,
}

#[derive(Serialize)]
struct Integration {
    name: String,
    configured: bool,
}

impl<'a, 'r, T> FromRequest<'a, 'r> for Ctx<T> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Ctx<T>, ()> {
        // TODO(richo) dedupe these cookie values across the two callsites
        let mut integrations = vec![];
        integrations.push(Integration {
            name: "dropbox".to_string(),
            configured: request.cookies().get("dropbox_token").is_some(),
        });
        integrations.push(Integration {
            name: "youtube".to_string(),
            configured: request.cookies().get("youtube_token").is_some(),
        });

        let mut ctx = Ctx {
            nav: nav::NAV_MAP.clone(),
            local: None,
            integrations,
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

