use std::fmt;

use rocket::request::Form;
use rocket::response::{Flash, Redirect};

use oauth2::prelude::*;
use oauth2::CsrfToken;

use crate::web::auth::WebUser;
use crate::web::db::DbConn;
use crate::web::models::{Integration, NewIntegration};
use crate::web::oauth::Oauth2Provider;

#[derive(FromForm, Debug)]
pub struct DisconnectForm {
    integration_id: i32,
    provider: Oauth2Provider,
}

#[post("/integration/disconnect", data = "<disconnect>")]
pub fn disconnect_integration(
    user: WebUser,
    disconnect: Form<DisconnectForm>,
    conn: DbConn,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    user.user
        .integration_by_id(disconnect.integration_id, &*conn)
        .map(|i| i.delete(&*conn))
        .map(|_| {
            Flash::success(
                Redirect::to("/"),
                format!(
                    "{} has been disconnected from your account.",
                    disconnect.provider.display_name()
                ),
            )
        })
        .map_err(|e| {
            warn!("{}", e);
            Flash::error(
                Redirect::to("/"),
                format!(
                    "{} could not be disconnected from your account.",
                    disconnect.provider.display_name()
                ),
            )
        })
}

#[derive(FromForm, Debug)]
pub struct ConnectForm {
    provider: Oauth2Provider,
}

#[post("/integration", data = "<connect>")]
pub fn connect_integration(
    mut user: WebUser,
    conn: DbConn,
    connect: Form<ConnectForm>,
) -> Redirect {
    let client = connect.provider.client();

    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random);

    user.session.insert(
        connect.provider.name().into(),
        csrf_state.secret().to_string().into(),
    );

    user.session.save(&*conn).unwrap();

    Redirect::to(authorize_url.as_str().to_string())
}

#[derive(FromForm)]
pub struct Oauth2Response {
    pub provider: Oauth2Provider,
    pub state: String,
    pub code: String,
    pub scope: Option<String>,
}

impl fmt::Debug for Oauth2Response {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Oauth2Response")
            .field("provider", &self.provider)
            .field("state", &self.state)
            .field("code", &"...")
            .field("scope", &self.scope)
            .finish()
    }
}

#[get("/integration/finish?<resp..>")]
pub fn finish_integration(
    user: WebUser,
    resp: Form<Oauth2Response>,
    conn: DbConn,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let integration: Option<Integration> = if user
        .session
        .data
        .get(resp.provider.name())
        .map(|state| state.as_str())
        != Some(Some(&resp.state))
    {
        warn!(
            "user {:?} oauth state didn't match for provider: {:?}",
            user.user.id, resp.provider
        );
        None
    } else {
        NewIntegration::new(&user.user, resp.provider.name(), &resp.code)
            .create(&*conn)
            .ok()
    };

    match integration {
        Some(_) => Ok(Flash::success(
            Redirect::to("/"),
            format!(
                "{} has been connected to your account.",
                resp.provider.display_name()
            ),
        )),
        None => Err(Flash::error(
            Redirect::to("/"),
            format!(
                "There was a problem connecting {} to your account.",
                resp.provider.display_name()
            ),
        )),
    }
}
