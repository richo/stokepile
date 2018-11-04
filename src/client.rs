use url::Url;
use failure::Error;

use serde_json;

use reqwest;
use reqwest::header::{HeaderMap, HeaderValue};

use messages;
use config;

/// A client to the web interface

#[derive(Fail, Debug)]
pub enum ClientError {
    #[fail(display = "Username or password was incorrect")]
    InvalidLogin,
    #[fail(display = "Server error: {}", _0)]
    ServerError(String),
}

pub type SessionToken = String;

#[derive(Debug)]
pub struct ArchiverClient {
    base: Url,
    client: reqwest::Client,
}

impl ArchiverClient {
    pub fn new(base: &str) -> Result<Self, Error> {
        let base = Url::parse(base)?;

        if !cfg!(debug_assertions) {
            if base.scheme() != "https" {
                return Err(format_err!("Non https urls not allowed in release builds"));
            }
        }

        Ok(ArchiverClient {
            base,
            client: reqwest::Client::new(),
        })
    }

    pub fn fetch_config(&self, token: config::AccessToken) -> Result<config::Config, Error> {
        let mut endpoint = self.base.clone();
        endpoint.set_path("/config");

        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::AUTHORIZATION, HeaderValue::from_str(&token.as_authorization_header())?);

        let mut resp = self.client.get(endpoint)
            // TODO(richo) we can actually reuse the web stuff for this
            .headers(headers)
            .send()?;

        if resp.status() == 500 {
            Err(ClientError::ServerError(resp.text()?))?;
        }

        config::Config::from_str(&resp.text()?)
    }

    pub fn login(&self, email: &str, password: &str) -> Result<SessionToken, Error> {
        let mut endpoint = self.base.clone();
        endpoint.set_path("/json/signin");

        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let payload = messages::JsonSignIn {
            email: email.into(),
            password: password.into(),
        };

        let mut resp = self.client.post(endpoint)
            // TODO(richo) we can actually reuse the web stuff for this
            .body(serde_json::to_string(&payload)?)
            .headers(headers)
            .send()?;

        if resp.status() == 500 {
            Err(ClientError::ServerError(resp.text()?))?;
        }

        let resp: messages::JsonSignInResp = resp.json()?;
        match resp {
            messages::JsonSignInResp::Token(s) => Ok(s),
            messages::JsonSignInResp::Error(error) => {
                warn!("{:?}", &error);
                Err(ClientError::InvalidLogin)?
            }
        }
    }
}
