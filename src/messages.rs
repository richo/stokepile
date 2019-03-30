/// This module contains message types which are shared between web and the client.

#[derive(Debug)]
pub enum Oauth2Provider {
    Dropbox,
    YouTube,
    GoogleDrive,
    Vimeo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonSignIn {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum JsonSignInResp {
    Token(String),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RefreshToken {
    Token(String),
    NotConfigured,
    Error(String),
}
