use rocket::http::RawStr;
use rocket::request::{Form, FromFormValue};
use rocket::response::{Flash, Redirect};

use crate::web::auth::WebUser;
use crate::web::db::DbConn;
use crate::web::models::{
    NewDevice,
};

#[derive(Debug)]
pub enum DeviceKind {
    Ptp,
    Flysight,
    MassStorage,
}

impl<'v> FromFormValue<'v> for DeviceKind {
    type Error = String;

    fn from_form_value(form_value: &'v RawStr) -> Result<DeviceKind, Self::Error> {
        let decoded = form_value.url_decode();
        match decoded {
            Ok(ref kind) if kind == "ptp" => Ok(DeviceKind::Ptp),
            Ok(ref kind) if kind == "flysight" => Ok(DeviceKind::Flysight),
            Ok(ref kind) if kind == "mass_storage" => Ok(DeviceKind::MassStorage),
            _ => Err(format!("unknown provider {}", form_value)),
        }
    }
}

impl DeviceKind {
    pub fn name(&self) -> &'static str {
        match self {
            DeviceKind::Ptp => "ptp",
            DeviceKind::Flysight => "flysight",
            DeviceKind::MassStorage => "mass_storage",
        }
    }
}

#[derive(Debug, FromForm)]
pub struct DeviceForm {
    name: String,
    kind: DeviceKind,
    identifier: String,
}

#[post("/device", data = "<device>")]
pub fn create_device(
    user: WebUser,
    conn: DbConn,
    device: Form<DeviceForm>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let row = NewDevice::new(
        &user.user,
        &device.name,
        device.kind.name(),
        &device.identifier,
    )
    .create(&*conn)
    .ok();
    match row {
        Some(_) => Ok(Flash::success(
            Redirect::to("/"),
            format!("{} was added to your configuration.", device.kind.name()),
        )),
        None => Err(Flash::error(
            Redirect::to("/"),
            format!(
                "There was a problem adding {} to your configuration.",
                device.kind.name()
            ),
        )),
    }
}

#[derive(Debug, FromForm)]
pub struct DeleteDeviceForm {
    device_id: i32,
    kind: DeviceKind,
}

#[post("/device/delete", data = "<device>")]
pub fn delete_device(
    user: WebUser,
    conn: DbConn,
    device: Form<DeleteDeviceForm>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    user.user
        .device_by_id(device.device_id, &*conn)
        .map(|i| i.delete(&*conn))
        .map(|_| {
            Flash::success(
                Redirect::to("/"),
                format!("{} has been removed from your account.", device.kind.name()),
            )
        })
        .map_err(|e| {
            warn!("{}", e);
            Flash::error(
                Redirect::to("/"),
                format!(
                    "{} could not be removed from your account.",
                    device.kind.name()
                ),
            )
        })
}
