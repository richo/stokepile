use bcrypt;
use diesel::prelude::*;

use super::*;
use crate::web::schema::users;

#[derive(Queryable, Debug, Serialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub notify_email: Option<String>,
    pub notify_pushover: Option<String>,
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
}

#[derive(Insertable, Debug)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub password: String,
}

impl<'a> NewUser<'a> {
    pub fn new(email: &'a str, password: &'a str) -> Self {
        let hashed_password = bcrypt::hash(&password, bcrypt::DEFAULT_COST).unwrap();

        NewUser {
            email: email,
            password: hashed_password,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<User> {
        use diesel::insert_into;

        insert_into(users::table)
            .values(self)
            .get_result::<User>(conn)
    }
}
