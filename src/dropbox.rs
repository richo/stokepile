/// This is a really small dropbox shim
///
/// If this library is useful, I'll consider fleshing it out into a whole thing

extern crate serde;
use serde::{Deserialize, Deserializer};
extern crate hex;
use hex::FromHex;
extern crate serde_json;
extern crate reqwest;

extern crate hyper;
use hyper::Headers;

use super::version;

use failure::Error;

use reqwest::header;

use std::path::Path;
use std::fs;
use std::io::Read;

header! { (DropboxAPIArg, "Dropbox-API-Arg") => [String] }

const DEFAULT_CHUNK_SIZE: usize = 4 * 1024 * 1024;

#[derive(Clone)]
pub struct DropboxFilesClient {
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
pub struct MetadataResponse {
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
    // TODO(richo) de-pub
    #[serde(deserialize_with = "hex_to_buffer")]
    pub content_hash: [u8; 32],
}

pub fn hex_to_buffer<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
  where D: Deserializer<'de>
{
  use serde::de::Error;
  String::deserialize(deserializer)
    .and_then(|string| <[u8; 32]  as FromHex>::from_hex(&string).map_err(|err| Error::custom(err.to_string())))
}

#[derive(Deserialize)]
#[derive(Debug)]
pub struct UploadMetadataResponse {
    name: String,
    path_lower: String,
    path_display: String,
    id: String,
    client_modified: String,
    server_modified: String,
    rev: String,
    size: usize,
    content_hash: String,
}

#[derive(Deserialize)]
#[derive(Debug)]
pub struct StartUploadSessionResponse {
    session_id: String,
}

#[derive(Serialize)]
#[derive(Debug)]
struct UploadSessionAppendRequest<'a> {
    cursor: &'a Cursor,
}

#[derive(Serialize)]
#[derive(Debug)]
struct UploadSessionFinishRequest<'a> {
    cursor: &'a Cursor,
    commit: &'a Commit<'a>,
}

#[derive(Serialize)]
#[derive(Debug)]
struct Cursor {
    session_id: String,
    offset: u64,
}

enum DropboxBody {
    JSON(Vec<u8>),
    Binary(Vec<u8>),
}

#[derive(Serialize)]
#[derive(Debug)]
struct Commit<'a> {
    path: &'a Path,
    mode: String,
}

pub struct UploadSession<'a> {
    client: &'a DropboxFilesClient,
    cursor: Cursor,
}

impl<'a> UploadSession<'a> {
    fn append(&mut self, data: &[u8]) -> Result<(), Error> {
        self.client.upload_session_append(data, &self.cursor)?;
        self.cursor.offset += data.len() as u64;
        Ok(())
    }

    fn finish(self, path: &Path) -> Result<UploadMetadataResponse, Error> {
        let commit = Commit {
            path: &path,
            mode: "overwrite".to_string(),
        };
        self.client.upload_session_finish(&[], self.cursor, commit)
    }
}


impl DropboxFilesClient {
    pub fn new(token: String) -> DropboxFilesClient {
        let client = reqwest::Client::new();
        DropboxFilesClient {
            token,
            user_agent: format!("archiver/{}", version::VERSION),
            client,
        }
    }

    fn request(&self, url: (&str, &str), body: DropboxBody, mut headers: Headers) -> Result<reqwest::Response, Error> {
        let url = format!("https://{}.dropboxapi.com/{}", url.0, url.1);

        headers.set(header::Authorization(header::Bearer { token: self.token.clone() }));
        headers.set(header::UserAgent::new(self.user_agent.clone()));
        headers.set(match &body {
            DropboxBody::JSON(_) => header::ContentType::json(),
            DropboxBody::Binary(_) => header::ContentType::octet_stream(),
        });

        self.client.post(&url)
        .body(match body {
            DropboxBody::JSON(vec) |
            DropboxBody::Binary(vec) => vec})
        .headers(headers)
        .send()
        .map_err(|e| format_err!("HTTP error: {:?}", e))
    }

    pub fn get_metadata(&self, path: &Path) -> Result<MetadataResponse, Error> {
        use self::DropboxBody::*;
        let req = serde_json::to_vec(&MetadataRequest { path: path.to_str().unwrap() })?;
        let headers = Headers::new();
        let mut res = self.request(("api", "2/files/get_metadata"), JSON(req), headers)?;
        // let meta: MetadataResponse = serde_json::from_str(&res.text()?)?;
        let text = res.text()?;
        eprintln!("{:?}", text);
        let meta: MetadataResponse = serde_json::from_str(&text)?;
        Ok(meta)
    }

    pub fn new_session<'a>(&'a self) -> Result<UploadSession<'a>, Error> {
        let id = self.start_upload_session()?.session_id;
        let cursor = Cursor {
            session_id: id,
            offset: 0,
        };

        Ok(UploadSession {
            client: self,
            cursor: cursor,
        })
    }

    pub fn upload_from_reader<T: Read>(&self, mut reader: T, remote_path: &Path) -> Result<UploadMetadataResponse, Error> {
        let id = self.start_upload_session()?;
        let mut buffer = vec![0; DEFAULT_CHUNK_SIZE];
        let mut cursor = Cursor {
            session_id: id.session_id,
            offset: 0,
        };

        loop {
            // There's more juggling than I would really like here but ok :(
            let read_bytes = reader.read(&mut buffer)?;
            if read_bytes == 0 {
                // We're probably at EOF? Hopefully?
                break
            }
            self.upload_session_append(&buffer[..read_bytes], &cursor)?;
            cursor.offset += read_bytes as u64;
        }

        let commit = Commit {
            path: &remote_path,
            mode: "overwrite".to_string(),
        };
        self.upload_session_finish(&[], cursor, commit)
    }

    fn start_upload_session<'a>(&self) -> Result<StartUploadSessionResponse, Error> {
        use self::DropboxBody::*;
        let headers = Headers::new();
        let mut res = self.request(("content", "2/files/upload_session/start"), Binary(vec![]), headers)?;
        let text = &res.text()?;
        let resp: StartUploadSessionResponse = serde_json::from_str(text)?;
        Ok(resp)
    }

    fn upload_session_append<'a>(&self, data: &[u8], cursor: &Cursor) -> Result<(), Error> {
        use self::DropboxBody::*;
        let req = serde_json::to_vec(&UploadSessionAppendRequest { cursor })?;
        let mut headers = Headers::new();
        headers.set(DropboxAPIArg(String::from_utf8(req)?));
        let mut res = self.request(("content", "2/files/upload_session/append_v2"), Binary(data.to_vec()), headers)?;
        res.text()?;
        Ok(())
    }

    fn upload_session_finish(&self, data: &[u8], cursor: Cursor, commit: Commit) -> Result<UploadMetadataResponse, Error> {
        use self::DropboxBody::*;
        let req = serde_json::to_vec(&UploadSessionFinishRequest {
                                        cursor: &cursor,
                                        commit: &commit })?;
        let mut headers = Headers::new();
        headers.set(DropboxAPIArg(String::from_utf8(req)?));
        let mut res = self.request(("content", "2/files/upload_session/finish"), Binary(data.to_vec()), headers)?;
        let text = res.text()?;
        let meta: UploadMetadataResponse = serde_json::from_str(&text)?;
        Ok(meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use sha2::{Sha256,Digest};
    use super::super::dropbox_content_hasher::DropboxContentHasher;

    #[test]
    #[ignore]
    fn test_fetches_metadata() {
        let client = DropboxFilesClient::new(env::var("ARCHIVER_TEST_DROPBOX_KEY").expect("Didn't provide test key"));
        client.get_metadata(Path::new("/15-01-01/rearcam/GOPR0001.MP4")).expect("Couldn't make test request");
    }

    #[test]
    #[ignore]
    fn test_uploads_large_file() {
        let client = DropboxFilesClient::new(env::var("ARCHIVER_TEST_DROPBOX_KEY").expect("Didn't provide test key"));
        let localfile = fs::File::open("/usr/share/dict/web2").expect("Couldn't open dummy dictionary");
        if let Err(e) = client.upload_from_reader(localfile, Path::new("/web2.txt")) {
            panic!("{:?}", e);
        }
    }

    #[test]
    #[ignore]
    fn test_roundtripped_content_hash_works() {
        let client = DropboxFilesClient::new(env::var("ARCHIVER_TEST_DROPBOX_KEY").expect("Didn't provide test key"));
        let localfile = b"yes hello";
        let hash = DropboxContentHasher::digest(&localfile[..]);
        eprintln!("hash!: {:?}", &hash);
        let path = Path::new("/archiver-test/hello.txt");
        if let Err(e) = client.upload_from_reader(&localfile[..], &path) {
            panic!("{:?}", e);
        }
        let metadata = client.get_metadata(&path).unwrap();
        assert_eq!(&metadata.content_hash[..], hash.as_slice());
    }

    #[test]
    #[ignore]
    fn test_uploaded_chunks_work() {
        fn inner() -> Result<(), Error> {
            let client = DropboxFilesClient::new(env::var("ARCHIVER_TEST_DROPBOX_KEY")?);
            let mut sess = client.new_session()?;
            sess.append(b"BUTTSBUTTS")?;
            sess.append(b"LOLOLOL")?;
            sess.finish(Path::new("/butts.txt"))?;
            Ok(())
        }

        if let Err(e) = inner() {
            panic!("{:?}", e);
        }
    }
}
