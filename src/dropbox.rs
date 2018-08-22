/// This is a really small dropbox shim
///
/// If this library is useful, I'll consider fleshing it out into a whole thing

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

struct LocalFile {
    file: fs::File,
    size: u64,
}

impl LocalFile {
    fn new<P: AsRef<Path>>(path: P) -> Result<LocalFile, Error> {
        Ok(LocalFile {
            file: fs::File::open(&path)?,
            size: fs::metadata(&path)?.len(),
        })
    }
}

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
struct UploadMetadataResponse {
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
struct StartUploadSessionResponse {
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

struct UploadSession<'a> {
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
    fn new(token: String) -> DropboxFilesClient {
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

    pub fn get_metadata<'a>(&self, path: &'a str) -> Result<MetadataResponse, Error> {
        use self::DropboxBody::*;
        let req = serde_json::to_vec(&MetadataRequest { path })?;
        let headers = Headers::new();
        let mut res = self.request(("api", "2/files/get_metadata"), JSON(req), headers)?;
        let meta: MetadataResponse = serde_json::from_str(&res.text()?)?;
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

    pub fn upload_large_file(&self, mut file: LocalFile, remote_path: &Path) -> Result<UploadMetadataResponse, Error> {
        let id = self.start_upload_session()?;
        let mut buffer = vec![0; DEFAULT_CHUNK_SIZE];
        let mut cursor = Cursor {
            session_id: id.session_id,
            offset: 0,
        };

        while cursor.offset < file.size {
            // There's more juggling than I would really like here but ok :(
            let read_bytes = file.file.read(&mut buffer)?;
            self.upload_session_append(&buffer[..read_bytes], &cursor)?;
            cursor.offset += read_bytes as u64;
        }
        assert_eq!(cursor.offset, file.size);

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
        let meta: UploadMetadataResponse = serde_json::from_str(&res.text()?)?;
        Ok(meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    #[ignore]
    fn test_fetches_metadata() {
        let client = DropboxFilesClient::new(env::var("ARCHIVER_TEST_DROPBOX_KEY").expect("Didn't provide test key"));
        client.get_metadata("/15-01-01/rearcam/GOPR0001.MP4").expect("Couldn't make test request");
    }

    #[test]
    #[ignore]
    fn test_uploads_large_file() {
        let client = DropboxFilesClient::new(env::var("ARCHIVER_TEST_DROPBOX_KEY").expect("Didn't provide test key"));
        let localfile = LocalFile::new("/usr/share/dict/web2").expect("Couldn't open dummy dictionary");
        if let Err(e) = client.upload_large_file(localfile, Path::new("/web2.txt")) {
            panic!("{:?}", e);
        }
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
