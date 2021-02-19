use bcrypt;
use diesel::prelude::*;

use super::*;
use crate::web::schema::users;
use crate::web::routes::settings::SettingsForm;
use crate::config::{MountableDeviceLocation, StagingConfig};

use rocket::http::RawStr;
use rocket::request::FromFormValue;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub notify_email: Option<String>,
    pub notify_pushover: Option<String>,
    pub staging_type: StagingKind,
    pub staging_data: Option<String>,
    pub preserve_device_files: bool,
    admin: bool,
    pub certificate: String,
    pub seal: Option<String>,
}

#[derive(Debug, DbEnum, Serialize, PartialEq)]
// We can't reuse this directly, without pulling all of the web stuff into the client, so instead
// we're going to have a mirror struct and some smoke unit tests that break if they're not kept in
// sync
pub enum StagingKind {
    None,
    Mountpoint,
    Label,
    Location,
}

impl<'v> FromFormValue<'v> for StagingKind {
    type Error = String;

    fn from_form_value(form_value: &'v RawStr) -> Result<StagingKind, Self::Error> {
        let decoded = form_value.url_decode();
        match decoded {
            Ok(ref kind) if kind == "None" => Ok(StagingKind::None),
            Ok(ref kind) if kind == "Label" => Ok(StagingKind::Label),
            Ok(ref kind) if kind == "Mountpoint" => Ok(StagingKind::Mountpoint),
            Ok(ref kind) if kind == "Location" => Ok(StagingKind::Location),
            _ => Err(format!("unknown staging_kind {}", form_value)),
        }
    }
}

impl User {
    pub fn by_credentials(conn: &PgConnection, email: &str, password: &str) -> Option<User> {
        use crate::web::schema::users::dsl::{email as user_email, users};

        if let Ok(user) = users.filter(user_email.eq(email)).get_result::<User>(conn) {
            if bcrypt::verify(password, &user.password).unwrap() {
                Some(user)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn by_email(conn: &PgConnection, email: &str) -> QueryResult<User> {
        use crate::web::schema::users::dsl::{email as user_email, users};

        users.filter(user_email.eq(email)).get_result::<User>(conn)
    }

    // TODO(richo) paginate
    pub fn all(conn: &PgConnection) -> QueryResult<Vec<User>> {
        use crate::web::schema::users::dsl::*;
        users.load::<User>(&*conn)
    }

    pub fn integrations(&self, conn: &PgConnection) -> QueryResult<Vec<Integration>> {
        use crate::web::schema::integrations::dsl::*;

        integrations
            .filter(user_id.eq(self.id))
            .load::<Integration>(conn)
    }

    pub fn devices(&self, conn: &PgConnection) -> QueryResult<Vec<Device>> {
        use crate::web::schema::devices::dsl::*;

        devices.filter(user_id.eq(self.id)).load::<Device>(conn)
    }

    pub fn keys(&self, conn: &PgConnection) -> QueryResult<Vec<Key>> {
        use crate::web::schema::keys::dsl::*;

        keys.filter(user_id.eq(self.id)).load::<Key>(conn)
    }

    pub fn customers(&self, conn: &PgConnection) -> QueryResult<Vec<Customer>> {
        use crate::web::schema::customers::dsl::*;

        customers.filter(user_id.eq(self.id)).load::<Customer>(conn)
    }

    pub fn customer_by_id(&self, conn: &PgConnection, customer_id: i32) -> QueryResult<Customer> {
        use crate::web::schema::customers::dsl::*;

        customers
            .filter(user_id.eq(self.id))
            .filter(id.eq(&customer_id))
            .get_result::<Customer>(conn)
    }

    pub fn equipment_by_id(&self, conn: &PgConnection, equipment_id: i32) -> QueryResult<Equipment> {
        use crate::web::schema::equipment::dsl::*;

        equipment
            .filter(user_id.eq(self.id))
            .filter(id.eq(&equipment_id))
            .get_result::<Equipment>(conn)
    }

    pub fn equipment(&self, conn: &PgConnection) -> QueryResult<Vec<Equipment>> {
        use crate::web::schema::equipment::dsl::*;

        equipment.filter(user_id.eq(self.id)).load::<Equipment>(conn)
    }

    pub fn integration_by_id(
        &self,
        integration_id: i32,
        conn: &PgConnection,
    ) -> QueryResult<Integration> {
        use crate::web::schema::integrations::dsl::*;

        integrations
            .filter(user_id.eq(self.id).and(id.eq(integration_id)))
            .get_result(conn)
    }

    pub fn device_by_id(&self, device_id: i32, conn: &PgConnection) -> QueryResult<Device> {
        use crate::web::schema::devices::dsl::*;

        devices
            .filter(user_id.eq(self.id).and(id.eq(device_id)))
            .get_result(conn)
    }

    pub fn key_by_id(&self, key_id: i32, conn: &PgConnection) -> QueryResult<Key> {
        use crate::web::schema::keys::dsl::*;

        keys.filter(user_id.eq(self.id).and(id.eq(key_id)))
            .get_result(conn)
    }

    pub fn staging(&self) -> Option<StagingConfig> {
        let loc = match &self.staging_data {
            Some(loc) => loc,
            None => return None,
        };
        let location = match &self.staging_type {
            StagingKind::None => return None,
            StagingKind::Label => MountableDeviceLocation::Label(loc.to_owned()),
            StagingKind::Mountpoint => MountableDeviceLocation::Mountpoint(loc.into()),
            StagingKind::Location => MountableDeviceLocation::Location(loc.into()),
        };
        Some(StagingConfig {
            location,
        })
    }

    // TODO(richo) This can leave you with a stale User object, unless we reload it in place.
    pub fn update_from_settings<S: SettingsUpdatable>(&self, settings: &S, conn: &PgConnection) -> QueryResult<usize> {
        settings.merge(&self, conn)
    }


    pub fn update_staging(&self, staging: &StagingConfig, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::update;
        use crate::web::schema::users::dsl::*;

        update(self)
            .set((
                    staging_type.eq(staging.kind_for_db()),
                    staging_data.eq(staging.data_for_db())
            ))
            .execute(conn)
    }

    pub fn is_admin(&self) -> bool {
        self.admin
    }

    pub fn promote(&self, conn: &PgConnection) -> QueryResult<usize> {
        self.set_admin(conn, true)
    }

    pub fn demote(&self, conn: &PgConnection) -> QueryResult<usize> {
        self.set_admin(conn, false)
    }

    fn set_admin(&self, conn: &PgConnection, value: bool) -> QueryResult<usize> {
        use diesel::update;
        use crate::web::schema::users::dsl::*;

        update(self)
            .set((
                    admin.eq(value),
            ))
            .execute(conn)
    }
}

#[derive(Insertable, Debug)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub password: String,
    admin: bool,
}

impl<'a> NewUser<'a> {
    pub fn new(email: &'a str, password: &'a str) -> Self {
        let hashed_password = bcrypt::hash(&password, bcrypt::DEFAULT_COST).unwrap();

        NewUser {
            email: email,
            password: hashed_password,
            admin: false,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<User> {
        use diesel::insert_into;

        insert_into(users::table)
            .values(self)
            .get_result::<User>(conn)
    }
}

pub trait SettingsUpdatable {
    fn merge(&self, user: &User, conn: &PgConnection) -> QueryResult<usize>;
}

impl SettingsUpdatable for crate::web::routes::rigging::settings::SettingsForm {
    fn merge(&self, user: &User, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::update;
        use crate::web::schema::users::dsl::*;

        update(user)
            .set((
                    certificate.eq(&self.certificate),
                    seal.eq(&self.seal),
            ))
            .execute(conn)
    }
}

impl SettingsUpdatable for crate::web::routes::settings::SettingsForm {
    fn merge(&self, user: &User, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::update;
        use crate::web::schema::users::dsl::*;

        let (ty, data) = self.staging()
            .map(|x| (x.kind_for_db(), Some(x.data_for_db())))
            .unwrap_or_else(|| (StagingKind::None, None));
        update(user)
            .set((
                    notify_email.eq(self.notification_email()),
                    notify_pushover.eq(self.notification_pushover()),
                    staging_type.eq(ty),
                    staging_data.eq(data),
                    preserve_device_files.eq(self.preserve_device_files)
            ))
            .execute(conn)
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MountableDeviceLocation;

    #[test]
    fn test_staging_kinds_are_in_sync() {
        // These don't have to run, we just want the definitions
        fn one_way(sk: StagingKind) {
            match sk {
                StagingKind::None => {},
                StagingKind::Label => {},
                StagingKind::Mountpoint => {},
                StagingKind::Location => {},
            }
        }

        fn other_way(ml: MountableDeviceLocation) {
            match ml {
                MountableDeviceLocation::Label(_) => {},
                MountableDeviceLocation::Mountpoint(_) => {},
                MountableDeviceLocation::Location(_) => {},
            }
        }
        // If you find yourself looking at this test, it's because one of those enums was updated
        // without the other.
    }
}
