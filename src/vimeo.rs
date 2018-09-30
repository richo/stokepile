use std::fs::File;

use tus;
use reqwest;
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use failure::Error;
use serde_json;

/// A client for the vimeo API
struct VimeoClient {
    token: String,
}

struct UploadHandle {
    // TODO(richo) native URL type
    url: String,
    complete: bool,
}

#[derive(Deserialize)]
struct CreateVideoResponse {
    upload: InnerCreateVideoResponse,
}

#[derive(Deserialize)]
struct InnerCreateVideoResponse {
    upload_link: String,
}

impl VimeoClient {
    /// Upload a file from the local filesystem to vimeo.
    pub fn upload_file(&self, file: File) -> Result<(), Error> {
        // First we find out how big the file is so we can create our video object upstream
        let size = file.metadata()?.len();
        // Then we create an upload handle
        let mut handle = self.create_upload_handle(size)?;

        Ok(())
    }

    fn create_upload_handle(&self, size: u64) -> Result<UploadHandle, Error> {
        let API_ENDPOINT = "https://api.vimeo.com/me/videos";
        let json = json!({
            "upload" : {
                "approach" : "tus",
                "size" : size,
            }
        });

        // Setup our headers
        let mut headers = self.default_headers();
        headers.insert(reqwest::header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(reqwest::header::ACCEPT, HeaderValue::from_static("application/vnd.vimeo.*+json;version=3.4"));

        // Create our request object
        let client = reqwest::Client::new();
        let text = client.post(API_ENDPOINT)
            .body(json.to_string())
            .headers(headers)
            .send()?
            .text()?;
        let response: CreateVideoResponse = serde_json::from_str(&text)
            .map_err(|e| format_err!("create_upload_handle: {}", text))?;
        Ok(UploadHandle {
            url: response.upload.upload_link,
            complete: false,
        })
    }

    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let mut authorization = HeaderValue::from_str(&format!("bearer {}", &self.token)).unwrap();
        authorization.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, authorization);
        headers
    }
}

impl Drop for UploadHandle {
    fn drop(&mut self) {
        if !self.complete {
            // TODO(richo) Destroy the file handle upstream
        }
    }
}
