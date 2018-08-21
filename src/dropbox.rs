/// This is a really small dropbox shim
///
/// If this library is useful, I'll consider fleshing it out into a whole thing

extern crate serde_json;
extern crate reqwest;

extern crate hyper;

use std::collections::BTreeMap;
use super::version;

use failure::Error;

use std::fmt;
use reqwest::header::{self, Header};

header! { (DropboxAPIArg, "Dropbox-API-Arg") => [String] }

struct DropboxFilesClient {
    token: String,
    user_agent: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct MetadataRequest<'a> {
    path: &'a str,
}

#[derive(Deserialize)]
#[derive(Debug)]
struct MetadataResponse {
    #[serde(rename = ".tag")]
    tag: String,
    name: String,
    id: String,
    client_modified: String,
    server_modified: String,
    rev: String,
    size: usize,
    path_lower: String,
    path_display: String,
    #[serde(skip)]
    sharing_info: (),
    #[serde(skip)]
    property_groups: (),
    content_hash: String,
}

#[derive(Deserialize)]
#[derive(Debug)]
struct StartUploadSessionResponse {
    session_id: String,
}

#[derive(Serialize)]
#[derive(Debug)]
struct UploadSessionAppendRequest {
    cursor: Cursor,
}

#[derive(Serialize)]
#[derive(Debug)]
struct Cursor {
    session_id: String,
    offset: usize,
}

impl DropboxFilesClient {
    fn new(token: String) -> DropboxFilesClient {
        let client = reqwest::Client::new();
        DropboxFilesClient {
            token,
            user_agent: format!("archiver/{}", version::VERSION),
            client,
        }
    }

    fn request(&self, url: (&str, &str), body: Option<Vec<u8>>, headers: &[&Header]) -> Result<reqwest::Response, Error> {
        let url = format!("https://{}.dropbox.com/{}", url.0, url.1);
        self.client.post(&url)
        .header(header::Authorization(header::Bearer { token: self.token.clone() }))
        .header(header::ContentType::json())
        .body(body.unwrap_or_else(|| vec![]))
        .send()
        .map_err(|e| format_err!("HTTP error: {:?}", e))
    }

    pub fn get_metadata<'a>(&self, path: &'a str) -> Result<MetadataResponse, Error> {
        let req = serde_json::to_vec(&MetadataRequest { path })?;
        let mut res = self.request(("api", "2/files/get_metadata"), Some(req), &[])?;
        let meta: MetadataResponse = serde_json::from_str(&res.text()?)?;
        Ok(meta)
    }

    fn start_upload_session<'a>(&self) -> Result<StartUploadSessionResponse, Error> {
        let mut res = self.request(("content", "2/files/upload_session/start"), Some(vec![b'{', b'}']), &[])?;
        let resp: StartUploadSessionResponse = serde_json::from_str(&res.text()?)?;
        Ok(resp)
    }

    fn upload_session_append<'a>(&self, session_id: String, data: &[u8], cursor: usize) -> Result<(), Error> {
        let req = serde_json::to_vec(&UploadSessionAppendRequest {
                                        cursor: Cursor {
                                            session_id: session_id,
                                            offset: cursor,
                                        }
        })?;
        let header = DropboxAPIArg(String::from_utf8(req)?);
        let mut res = self.request(("content", "2/files/upload_session/start"), Some(data.to_vec()), &[&header])?;
        res.text()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    #[ignore]
    fn test_sample_request() {
        let client = DropboxFilesClient::new(env::var("ARCHIVER_TEST_DROPBOX_KEY").expect("Didn't provide test key"));
        let res = client.get_metadata("/15-01-01/rearcam/GOPR0001.MP4").expect("Couldn't make test request");
    }
}
