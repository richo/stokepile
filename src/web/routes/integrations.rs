use failure::Error;
use rocket::form::Form;
use rocket::response::{Flash, Redirect};

use oauth2::prelude::*;
use oauth2::CsrfToken;

use crate::web::oauth;
use crate::web::auth::WebUser;
use crate::web::db::DbConn;
use crate::web::models::{Integration, NewIntegration};
use crate::messages::Oauth2Provider;

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

#[derive(FromForm, RedactedDebug)]
pub struct Oauth2Response {
    pub provider: Oauth2Provider,
    pub state: String,
    #[redacted]
    pub code: String,
    pub scope: Option<String>,
}

#[get("/integration/finish?<resp..>")]
pub fn finish_integration(
    user: WebUser,
    resp: Form<Oauth2Response>,
    conn: DbConn,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let integration: Result<Integration, Error> = if user
        .session
        .data
        .get(resp.provider.name())
        .and_then(|state| state.as_str())
        != Some(&resp.state)
    {
        warn!(
            "user {:?} oauth state didn't match for provider: {:?}",
            user.user.id, resp.provider
        );
        Err(format_err!("Oauth state didn't match"))
    } else {
        oauth::exchange_oauth_code(&resp.provider, &resp.code)
            .and_then(|(access_token, refresh_token)| {
                let refresh_token = refresh_token.as_ref().map(String::as_str);
                NewIntegration::new(&user.user, resp.provider.name(), &access_token, refresh_token)
                .create(&*conn)
                .map_err(|e| e.into())
        })
    };

    match integration {
        Ok(_) => Ok(Flash::success(
            Redirect::to("/"),
            format!(
                "{} has been connected to your account.",
                resp.provider.display_name()
            ),
        )),
        Err(e) => {
            error!("Error creating integration for user {:?}: {:?}",
                   &user.user.id, e);

            Err(Flash::error(
            Redirect::to("/"),
            format!(
                "There was a problem connecting {} to your account.",
                resp.provider.display_name())))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;
    use crate::web::oauth;

    use rocket::http::{ContentType, Header, Status};

    client_for_routes!(config: connect_integration, disconnect_integration, finish_integration => client);

    #[test]
    fn test_connect_integration() {
        init_env();

        let client = client();
        create_user(&client, "test@email.com", "p@55w0rd");
        let session_cookie = signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let req = client
            .post("/integration")
            .header(ContentType::Form)
            .body(r"provider=dropbox");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        assert!(response
            .headers()
            .get_one("Location")
            .unwrap()
            .starts_with(oauth::DROPBOX_CONFIG.auth_url.as_str()));

        let session = session_from_cookie(&client, session_cookie).unwrap();
        assert!(session.data.get("dropbox").unwrap().is_string());
    }

    #[test]
    fn test_disconnect_integration() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let integration_id = {
            let conn = db_conn(&client);

            NewIntegration::new(&user, "dropbox", "test_oauth_token", None)
                .create(&*conn)
                .unwrap()
                .id
        };

        let req = client
            .post("/integration/disconnect")
            .header(ContentType::Form)
            .body(format!(
                "provider=dropbox&integration_id={}",
                integration_id
            ));

        let response = req.dispatch();

        assert_eq!(response.status(), Status::SeeOther);

        let conn = db_conn(&client);
        assert_eq!(user.integrations(&*conn).unwrap().len(), 0);
    }

    #[test]
    fn test_finish_integration() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        let session_cookie = signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let mut session = session_from_cookie(&client, session_cookie.clone()).unwrap();

        {
            let conn = db_conn(&client);
            session.insert("dropbox".to_string(), "test_csrf_token".into());
            session.save(&*conn).unwrap();
        }

        let req = client
            .get("/integration/finish?provider=dropbox&state=test_csrf_token&code=test_oauth_token")
            .header(Header::new("Cookie", session_cookie.clone()));

        let response = req.dispatch();

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/"));

        let conn = db_conn(&client);

        let integrations = user.integrations(&*conn)
                .unwrap();
        let integration = integrations
                .first()
                .unwrap();

        assert_eq!(
            &integration
                .access_token,
            "test_access_token"
        );
        assert_eq!(
            &integration
                .refresh_token,
            &Some("test_refresh_token".into())
        );
    }

    #[test]
    fn test_connect_integration_doesnt_stomp_on_sessions() {
        init_env();

        let client1 = client();
        let client2 = client();
        let _u1 = create_user(&client1, "test1@email.com", "p@55w0rd");
        let _u2 = create_user(&client2, "test2@email.com", "p@55w0rd");

        let s1 = signin(&client1, "test1%40email.com", "p%4055w0rd").unwrap();
        let s2 = signin(&client2, "test2%40email.com", "p%4055w0rd").unwrap();

        let session1 = session_from_cookie(&client1, s1.clone()).unwrap();
        let session2 = session_from_cookie(&client2, s2.clone()).unwrap();

        assert!(
            session1.user_id != session2.user_id,
            "User IDs have been tampered with"
        );

        let req = client1
            .post("/integration")
            .header(ContentType::Form)
            .body(r"provider=dropbox");

        let response = req.dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        assert!(response
            .headers()
            .get_one("Location")
            .unwrap()
            .starts_with(oauth::DROPBOX_CONFIG.auth_url.as_str()));

        let session1 = session_from_cookie(&client1, s1.clone()).unwrap();
        let session2 = session_from_cookie(&client2, s2.clone()).unwrap();

        assert!(session1.data.get("dropbox").unwrap().is_string());
        assert!(session2.data.get("dropbox").is_none());
        assert!(
            session1.user_id != session2.user_id,
            "User IDs have been tampered with"
        );
    }
}
