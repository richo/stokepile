/// This module contains message types which are shared between web and the client.

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
pub struct SendNotification<'a> {
    pub message: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SendNotificationResp {
    Sent,
    Error(String),
}
