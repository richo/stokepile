use url::Url;
use failure::Error;
///
/// A client to the web interface

struct ArchiverClient {
    base: Url,
}

impl Default for ArchiverClient {
    fn default() -> Self {
        ArchiverClient::new("https://onatopp.psych0tik.net").unwrap()
    }
}

impl ArchiverClient {
    pub fn new(base: &str) -> Result<Self, Error> {
        Ok(ArchiverClient {
            base: Url::parse(base)?,
        })
    }
}
