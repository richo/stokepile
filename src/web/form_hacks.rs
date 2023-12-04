// lifted from https://github.com/SergioBenitez/Rocket/issues/602#issuecomment-380497269

use std::ops::Deref;
use uuid::{self, Uuid};
use rocket::http::RawStr;
use rocket::request::FromParam;

#[derive(Debug, Serialize)]
pub struct UuidParam(Uuid);

impl<'r> FromParam<'r> for UuidParam {
    type Error = uuid::Error;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
       Uuid::parse_str(param).map(UuidParam)
    }
}

impl Deref for UuidParam {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
