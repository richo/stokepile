// lifted from https://github.com/SergioBenitez/Rocket/issues/602#issuecomment-380497269

use std::ops::Deref;
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
