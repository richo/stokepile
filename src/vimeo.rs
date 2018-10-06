use std::fs::File;
use std::io::Read;

use tus;
use reqwest;
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use failure::Error;
use serde_json;

use storage::{StorageAdaptor, StorageStatus};
use staging;

/// A client for the vimeo API
pub struct VimeoClient {
    token: String,
}

struct UploadHandle {
    // TODO(richo) native URL type
    url: String,
    complete: bool,
}

#[derive(Deserialize)]
struct CreateVideoResponse {
    uri: String,
    resource_key: String,
    upload: InnerUploadCreateVideoResponse,
}

#[derive(Deserialize)]
struct InnerUploadCreateVideoResponse {
    upload_link: String,
}

impl VimeoClient {
    /// Create a new VimeoClient authenticated by `token`.
    pub fn new(token: String) -> VimeoClient {
        VimeoClient {
            token,
        }
    }

    fn create_upload_handle(&self, name: &str, size: u64) -> Result<UploadHandle, Error> {
        let api_endpoint = "https://api.vimeo.com/me/videos";
        let json = json!({
            "upload" : {
                "approach" : "tus",
                "size" : size,
            },
            "name" : name,
        });

        // Setup our headers
        let mut headers = self.default_headers(size);
        headers.insert(reqwest::header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(reqwest::header::ACCEPT, HeaderValue::from_static("application/vnd.vimeo.*+json;version=3.4"));

        // Create our request object
        let client = reqwest::Client::new();
        let text = client.post(api_endpoint)
            .body(json.to_string())
            .headers(headers)
            .send()?
            .text()?;
        let response: CreateVideoResponse = serde_json::from_str(&text)
            .map_err(|e| format_err!("create_upload_handle: {:?} {}", e, text))?;
        Ok(UploadHandle {
            url: response.upload.upload_link,
            complete: false,
        })
    }

    fn default_headers(&self, size: u64) -> HeaderMap {
        let mut headers = tus::default_headers(size);
        let mut authorization = HeaderValue::from_str(&format!("bearer {}", &self.token)).unwrap();
        authorization.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, authorization);
        headers
    }
}

impl StorageAdaptor<File> for VimeoClient {
    fn already_uploaded(&self, manifest: &staging::UploadDescriptor) -> bool {
        // TODO(richo) Actually figure out how to check if we've already done this
        false
    }

    /// Upload a file from the local filesystem to vimeo.
    fn upload(&self, file: File, manifest: &staging::UploadDescriptor) -> Result<StorageStatus, Error> {
        // First we find out how big the file is so we can create our video object upstream
        let size = file.metadata()?.len();
        // Then we create an upload handle
        let handle = self.create_upload_handle(&manifest.staging_name(), size)?;

        let headers = self.default_headers(size);
        let tusclient = tus::Client::new(&handle.url, headers);
        let sent = tusclient.upload(file)?;

        // TODO(richo) look through sent and confirm it really sent

        Ok(StorageStatus::Success)
    }

    fn name(&self) -> String {
        "vimeo".to_string()
    }
}

impl Drop for UploadHandle {
    fn drop(&mut self) {
        if !self.complete {
            // TODO(richo) Destroy the file handle upstream
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    #[ignore]
    fn test_creates_upload_handle() {
        let client = VimeoClient::new(
            env::var("ARCHIVER_TEST_VIMEO_KEY").expect("Didn't provide test key"),
        );
        let handle = client.create_upload_handle("test_video.mp4", 1024).expect("Couldn't create upload handle");
        assert!(handle.url.starts_with("https://files.tus.vimeo.com"), "Handle url not rooted at vimeo.com");
        assert_eq!(handle.complete, false);
    }

    #[test]
    #[ignore]
    fn test_uploads_file_to_vimeo() {
        let client = VimeoClient::new(
            env::var("ARCHIVER_TEST_VIMEO_KEY").expect("Didn't provide test key"),
        );
        let fh = File::open("/tmp/test.mp4").expect("Couldn't open video");
        let desc = staging::UploadDescriptor::test_descriptor();
        client.upload(fh, &desc).expect("Could not upload file");
    }
}
