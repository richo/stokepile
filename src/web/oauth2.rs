use oauth2::basic::BasicClient;
use oauth2::prelude::*;
use oauth2::{AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
             TokenUrl};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;

#[derive(Clone)]
pub struct Oauth2Config {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub auth_url: AuthUrl,
    pub token_url: TokenUrl,
    // TODO(richo) scopes?
    pub redirect_url: RedirectUrl,
}

#[derive(FromForm, Debug)]
pub struct Oauth2Response {
    pub state: String,
    pub code: String,
}

impl Oauth2Config {
    /// Creates a Oauth2Config configured for Dropbox, panicing on many types of failure, since
    /// they are all unrecoverable.
    pub fn dropbox() -> Oauth2Config {
        let client_id = ClientId::new(env::var("ARCHIVER_DROPBOX_APP_KEY")
                                      .expect("Missing the ARCHIVER_DROPBOX_APP_KEY environment variable."));
        let client_secret = ClientSecret::new(env::var("ARCHIVER_DROPBOX_APP_SECRET")
                                              .expect("Missing the GITHUB_CLIENT_SECRET environment variable."));
        let auth_url = AuthUrl::new(Url::parse("https://www.dropbox.com/oauth2/authorize")
                                    .expect("Invalid authorization endpoint URL"));
        let token_url = TokenUrl::new(Url::parse("https://www.dropbox.com/oauth2/token")
                                      .expect("Invalid token endpoint URL"));
        let redirect_url = RedirectUrl::new(Url::parse("http://localhost:8000/dropbox/finish")
                                            .expect("Invalid redirect URL"));

        Oauth2Config {
            client_id,
            client_secret,
            auth_url,
            token_url,
            // TODO(richo) stuff this in config somewhere
            redirect_url,
        }
    }

    pub fn client(&self) -> BasicClient {
        let Oauth2Config {
            client_id,
            client_secret,
            auth_url,
            token_url,
            redirect_url,
        } = self;
        BasicClient::new(
            client_id.clone(),
            Some(client_secret.clone()),
            auth_url.clone(),
            Some(token_url.clone())
        ).set_redirect_url(redirect_url.clone())
    }
}
