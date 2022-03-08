// lifted from https://github.com/SergioBenitez/Rocket/issues/602#issuecomment-380497269

use std::ops::Deref;

use uuid::{self, Uuid};
use rocket::http::RawStr;
use rocket::request::FromParam;

use chrono::NaiveDate;
use rocket::http::RawStr;
use rocket::request::FromFormValue;

#[derive(Debug, Serialize)]
pub struct UuidParam(Uuid);

impl<'r> FromParam<'r> for UuidParam {
    type Error = uuid::Error;

    fn from_param(param: &'r RawStr) -> Result<Self, Self::Error> {
       Uuid::parse_str(param).map(UuidParam)
    }
}

impl Deref for UuidParam {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

use chrono::NaiveDate;
use rocket::http::RawStr;
use rocket::request::FromFormValue;

#[derive(Debug, Serialize)]
pub struct NaiveDateForm(NaiveDate);

impl<'v> FromFormValue<'v> for NaiveDateForm {
    type Error = chrono::format::ParseError;

    fn from_form_value(value: &'v RawStr) -> Result<NaiveDateForm, chrono::format::ParseError> {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(|d| NaiveDateForm(d))
    }
}

impl Deref for NaiveDateForm {
    type Target = NaiveDate;

    fn deref(&self) -> &NaiveDate {
        &self.0
    }
}
