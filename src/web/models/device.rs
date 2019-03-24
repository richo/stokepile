use diesel::prelude::*;

use super::*;
use crate::web::schema::devices;

use crate::config;
use crate::config::{FlysightConfig, GoproConfig, MassStorageConfig, MountableDeviceLocation};

#[derive(Identifiable, Queryable, Associations, Debug, Serialize)]
#[belongs_to(User)]
pub struct Device {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub kind: String,
    /// This represents the unique identifier of this device. Eg, a single discriminant through
    /// with combined with the type, you can concretely say whether or not a candidate device is
    /// this one.
    pub identifier: String,
}

impl From<Device> for config::DeviceConfig {
    fn from(device: Device) -> Self {
        match &device.kind[..] {
            "ptp" => config::DeviceConfig::Gopro(GoproConfig {
                name: device.name,
                serial: device.identifier,
            }),
            "mass_storage" => {
                config::DeviceConfig::MassStorage(MassStorageConfig {
                    name: device.name,
                    // TODO(richo) add a metadata field and store this there
                    extensions: vec!["mp4".into()],
                    location: MountableDeviceLocation::from_label(device.identifier),
                })
            }
            "flysight" => config::DeviceConfig::Flysight(FlysightConfig {
                name: device.name,
                location: MountableDeviceLocation::from_label(device.identifier),
            }),
            kind => {
                // This feels sound with the overlapping borrows, revisit?
                config::DeviceConfig::UnknownDevice(kind.to_string())
            }
        }
    }
}

impl Device {
    pub fn by_id(&self, device_id: i32, conn: &PgConnection) -> QueryResult<Device> {
        use crate::web::schema::devices::dsl::*;

        devices.filter(id.eq(device_id)).get_result::<Device>(conn)
    }

    pub fn delete(&self, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::delete;

        delete(self).execute(conn)
    }
}

#[derive(Insertable, Debug)]
#[table_name = "devices"]
pub struct NewDevice<'a> {
    pub user_id: i32,
    pub name: &'a str,
    pub kind: &'a str,
    pub identifier: &'a str,
}

impl<'a> NewDevice<'a> {
    pub fn new(user: &User, name: &'a str, kind: &'a str, identifier: &'a str) -> Self {
        NewDevice {
            user_id: user.id,
            kind,
            name,
            identifier,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Device> {
        use diesel::insert_into;

        insert_into(devices::table)
            .values(self)
            .get_result::<Device>(conn)
    }
}
