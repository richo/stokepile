/// This module contains message types which are shared between web and the client.
use failure::Error;

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

impl JsonSignInResp {
    pub fn into_result(self) -> Result<String, Error> {
        // TODO(richo) can we just serialize Error's ?
        match self {
            JsonSignInResp::Token(token) => Ok(token),
            JsonSignInResp::Error(error) => Err(format_err!("{:?}", error)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum RefreshToken {
    Token(String),
    NotConfigured,
    Error(String),
}


#[derive(Serialize, Deserialize, Debug)]
pub struct SendNotification {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SendNotificationResp {
    Sent,
    NotConfigured,
    Error(String),
}
