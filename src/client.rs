use failure::Error;
use url::Url;

use serde_json;

use reqwest;
use reqwest::header::{HeaderMap, HeaderValue};

use crate::config;
use crate::messages;

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
    token: Option<config::AccessToken>,
}

impl ArchiverClient {
    #[allow(clippy::collapsible_if)]
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
            token: None,
        })
    }

    pub fn load_token(&mut self) -> Result<(), Error> {
        info!("Loading access token into client");
        let token = config::AccessToken::load()?;
        self.token = Some(token);
        Ok(())
    }

    pub fn fetch_config(&self) -> Result<config::Config, Error> {
        let mut endpoint = self.base.clone();
        endpoint.set_path("/config");

        let headers = self.add_authorization(HeaderMap::new())?;

        let mut resp = self
            .client
            .get(endpoint)
            // TODO(richo) we can actually reuse the web stuff for this
            .headers(headers)
            .send()?;

        if resp.status() == 500 {
            Err(ClientError::ServerError(resp.text()?))?;
        }

        Ok(resp.text()?.parse()?)
    }

    pub fn send_notification(&self, msg: &str) -> Result<(), Error> {
        let mut endpoint = self.base.clone();
        endpoint.set_path("/notification/send");

        let headers = self.json_content_type(
            self.add_authorization(
                HeaderMap::new())?);

        let payload = messages::SendNotification {
            message: msg.into(),
        };

        let mut resp = self
            .client
            .post(endpoint)
            // TODO(richo) we can actually reuse the web stuff for this
            .body(serde_json::to_string(&payload)?)
            .headers(headers)
            .send()?;

        if resp.status() == 500 {
            Err(ClientError::ServerError(resp.text()?))?;
        }

        let resp: messages::SendNotificationResp = resp.json()?;
        match resp {
            messages::SendNotificationResp::Sent |
            messages::SendNotificationResp::NotConfigured => {
                Ok(())
            },
            messages::SendNotificationResp::Error(e) => {
                Err(format_err!("{:?}", e))
            },
        }
    }

    pub fn login(&self, email: &str, password: &str) -> Result<SessionToken, Error> {
        let mut endpoint = self.base.clone();
        endpoint.set_path("/json/signin");

        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        let payload = messages::JsonSignIn {
            email: email.into(),
            password: password.into(),
        };

        let mut resp = self
            .client
            .post(endpoint)
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

    fn add_authorization(&self, mut headers: HeaderMap) -> Result<HeaderMap, Error> {
        match &self.token {
            Some(token) => {
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    HeaderValue::from_str(&token.as_authorization_header())?,
                    );
                Ok(headers)
            },
            None => {
                bail!("Attempted to call an authenticated method without a token set");
            }
        }
    }

    fn json_content_type(&self, mut headers: HeaderMap) -> HeaderMap {
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers
    }
}
