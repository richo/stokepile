use std::convert::{TryFrom, TryInto};

use chrono::prelude::*;
use rocket::http::RawStr;
use rocket::request::{FromForm, FormItems};

use crate::web::form_hacks::NaiveDateForm;

#[derive(Debug)]
pub enum EquipmentFormError {
    Parsing(std::str::Utf8Error),
    DateParsing(chrono::ParseError),
    MissingField,
    ExtraFields,

    ComponentNotProvided,
}

// TODO(richo) Support for an optional AAD
macro_rules! equipment_form_members {
    ( $item:expr, $( $name:ident => $struct:expr ),+ ) => (
        match $item.key.as_str() {
            // TODO(richo) Should we just stash the potentiaal errors in the proto object for a
            // while and barf later trying to convert?
            $(
                concat!(stringify!($name), "_manufacturer") => {
                    $struct.manufacturer = Some($struct.convert_value($item.value)?);
                },
                concat!(stringify!($name), "_model") => {
                    $struct.model = Some($struct.convert_value($item.value)?);
                },
                concat!(stringify!($name), "_serial") => {
                    $struct.serial = Some($struct.convert_value($item.value)?);
                },
                concat!(stringify!($name), "_dom") => {
                    $struct.dom = Some($struct.convert_value($item.value)?);
                },
            )+
                // TODO(richo)
            // _ if strict => return Err(EquipmentFormError::ExtraFields),
            field => { debug!("Got an extra field: {:?} => {:?}", &field, &$item.value); },
        }
    );
}

impl<'f> FromForm<'f> for NewEquipmentForm {
    // In practice, we'd use a more descriptive error type.
    type Error = EquipmentFormError;

    fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<NewEquipmentForm, EquipmentFormError> {
        let mut container = ProtoComponent::default();
        let mut reserve = ProtoComponent::default();
        let mut aad = ProtoComponent::default();

        for item in items {
            equipment_form_members!(item,
                container => container,
                reserve => reserve,
                aad => aad);
        }

        // deal with the aad specifically since we want to throw away kind kind of error
        let aad = match aad.try_into() {
            Ok(aad) => Some(aad),
            Err(EquipmentFormError::ComponentNotProvided) => None,
            Err(e) => Err(e)?,
        };

        Ok(NewEquipmentForm {
            container: container.try_into()?,
            reserve: reserve.try_into()?,
            aad,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NewEquipmentForm {
    #[serde(flatten)]
    pub container: Component,

    #[serde(flatten)]
    pub reserve: Component,

    #[serde(flatten)]
    pub aad: Option<Component>,

    // TODO(richo) notes? Bury the notes in the data field?
}

#[derive(FromForm, Debug, Serialize)]
pub struct NewCustomerForm {
    pub name: String,
    pub address: String,
    pub phone_number: String,
    pub email: String,
}

#[derive(Debug, Serialize, FromForm)]
pub struct RepackForm {
    pub place: String,
    pub certificate: String,
    pub seal: String,
    pub service: String,
    pub date: NaiveDateForm,
}

#[derive(Debug, Serialize)]
pub struct Component {
    pub manufacturer: String,
    pub model: String,
    pub serial: String,
    pub dom: NaiveDate,
}

#[derive(Debug, Default)]
pub struct ProtoComponent {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub dom: Option<String>,
}

trait UnpackValue {
    fn convert_value(&self, value: &RawStr) -> Result<String, EquipmentFormError> {
        value.url_decode().map_err(|e| EquipmentFormError::Parsing(e))
    }

    fn convert_date(value: &str) -> Result<NaiveDate, EquipmentFormError> {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|e| EquipmentFormError::DateParsing(e))
    }
}

impl UnpackValue for ProtoComponent {}

impl TryFrom<ProtoComponent> for Component {
    type Error = EquipmentFormError;

    fn try_from(value: ProtoComponent) -> Result<Self, Self::Error> {
        match (value.manufacturer, value.model, value.serial, value.dom) {
            (Some(manufacturer), Some(model), Some(serial), _) if
                manufacturer == "" && model == "" && serial == "" =>  {
                    Err(EquipmentFormError::ComponentNotProvided)
                },
                (Some(manufacturer), Some(model), Some(serial), Some(dom)) =>  {
                    Ok(Component {
                        manufacturer,
                        model,
                        serial,
                        dom: ProtoComponent::convert_date(&dom)?,
                    })
            },
            _ => {
                // TODO(richo) Good error
                Err(EquipmentFormError::MissingField)
            }
        }
    }
}
