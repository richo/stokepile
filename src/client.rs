use url::Url;
use failure::Error;

use reqwest;
use reqwest::header::{HeaderMap, HeaderValue};

/// A client to the web interface

#[derive(Fail, Debug)]
pub enum ClientError {
    #[fail(display = "Username or password was incorrect")]
    InvalidLogin,
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

        if cfg!(debug_assertions) {
            if base.scheme() != "https" {
                return Err(format_err!("Non https urls not allowed in release builds"));
            }
        }

        Ok(ArchiverClient {
            base,
            client: reqwest::Client::new(),
        })
    }

    pub fn login(&self, email: &str, password: &str) -> Result<SessionToken, Error> {
        let mut endpoint = self.base.clone();
        endpoint.set_path("/signin");

        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded"));

        let resp = self.client.post(endpoint)
            // TODO(richo) we can actually reuse the web stuff for this
            .body(format!(
                "email={}&password={}",
                email, password))
            .headers(headers)
            .send()?;



    }
}
