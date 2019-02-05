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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;

    use rocket::http::{ContentType, Status};

    client_for_routes!(create_device, delete_device => client);

    #[test]
    fn test_create_devices() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        for (name, kind, identifier) in &[
            ("gopro5", "ptp", "C123456"),
            ("comp", "flysight", "/mnt/flysight"),
            ("sdcard", "mass_storage", "/media/sdcard"),
        ] {
            let req = client
                .post("/device")
                .header(ContentType::Form)
                .body(format!(
                    "name={}&kind={}&identifier={}",
                    name, kind, identifier
                ));

            let response = req.dispatch();

            assert_eq!(response.status(), Status::SeeOther);
            assert_eq!(response.headers().get_one("Location"), Some("/"));
        }

        let conn = db_conn(&client);

        let devices = user.devices(&*conn).unwrap();
        assert_eq!(devices.len(), 3);
    }

    #[test]
    fn test_invalid_device_type() {
        init_env();

        let client = client();
        create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let add_device = |kind, name| {
            let req = client
                .post("/device")
                .header(ContentType::Form)
                .body(format!("kind={}&identifier={}", &kind, &name));

            let response = req.dispatch();

            assert_eq!(response.status(), Status::UnprocessableEntity);
        };

        add_device("nonexistant", "gopro5");
    }

    #[test]
    fn test_delete_devices() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let device_id = {
            let conn = db_conn(&client);

            NewDevice::new(&user, "gopro", "ptp", "test_gopro")
                .create(&*conn)
                .unwrap()
                .id
        };

        {
            let conn = db_conn(&client);

            assert_eq!(user.devices(&*conn).unwrap().len(), 1);
        }

        let req = client
            .post("/device/delete")
            .header(ContentType::Form)
            .body(format!("kind=ptp&device_id={}", device_id));

        let response = req.dispatch();

        assert_eq!(response.status(), Status::SeeOther);

        let conn = db_conn(&client);
        assert_eq!(user.devices(&*conn).unwrap().len(), 0);
    }
}
