#![allow(proc_macro_derive_resolution_fallback)]

use serde_json;

use bcrypt;
use diesel::prelude::*;
use rand;

use web::schema::devices;
use web::schema::integrations;
use web::schema::sessions;
use web::schema::users;

#[derive(Queryable, Debug, Serialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
}

impl User {
    pub fn by_credentials(conn: &PgConnection, email: &str, password: &str) -> Option<User> {
        use web::schema::users::dsl::{email as user_email, users};

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
        use web::schema::integrations::dsl::*;

        integrations
            .filter(user_id.eq(self.id))
            .load::<Integration>(conn)
    }

    pub fn devices(&self, conn: &PgConnection) -> QueryResult<Vec<Device>> {
        use web::schema::devices::dsl::*;

        devices
            .filter(user_id.eq(self.id))
            .load::<Device>(conn)
    }

    pub fn integration_by_id(
        &self,
        integration_id: i32,
        conn: &PgConnection,
    ) -> QueryResult<Integration> {
        use web::schema::integrations::dsl::*;

        integrations
            .filter(user_id.eq(self.id).and(id.eq(integration_id)))
            .get_result(conn)
    }

    pub fn device_by_id(
        &self,
        device_id: i32,
        conn: &PgConnection,
    ) -> QueryResult<Device> {
        use web::schema::devices::dsl::*;

        devices
            .filter(user_id.eq(self.id).and(id.eq(device_id)))
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

#[derive(Identifiable, Queryable, Associations, Debug, AsChangeset, PartialEq)]
#[belongs_to(User)]
pub struct Session {
    pub id: String,
    pub user_id: i32,
    pub data: serde_json::Value,
}

impl Session {
    pub fn by_id(conn: &PgConnection, session_id: &str) -> QueryResult<(Self, User)> {
        use web::schema::sessions::dsl::*;
        use web::schema::users;

        sessions
            .inner_join(users::table)
            .filter(id.eq(session_id))
            .get_result::<(Session, User)>(conn)
    }

    pub fn insert(&mut self, key: String, value: serde_json::Value) -> Option<serde_json::Value> {
        let data = self.data.as_object_mut()?;
        data.insert(key, value)
    }

    pub fn save(&self, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::update;
        use web::schema::sessions::dsl::*;

        update(self).set(data.eq(&self.data)).execute(conn)
    }

    pub fn delete(&self, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::delete;

        delete(self).execute(conn)
    }
}

#[derive(Insertable, Debug)]
#[table_name = "sessions"]
pub struct NewSession {
    pub id: String,
    pub user_id: i32,
}

fn generate_session_id() -> String {
    let (x, y) = rand::random::<(u64, u64)>();
    format!("{:x}{:x}", x, y)
}

impl NewSession {
    pub fn new(user: &User) -> Self {
        NewSession {
            id: generate_session_id(),
            user_id: user.id,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Session> {
        use diesel::insert_into;

        insert_into(sessions::table)
            .values(self)
            .get_result::<Session>(conn)
    }
}

#[derive(Identifiable, Queryable, Associations, Debug)]
#[belongs_to(User)]
pub struct Integration {
    pub id: i32,
    pub user_id: i32,
    pub provider: String,
    pub access_token: String,
}

impl Integration {
    pub fn by_id(&self, integration_id: i32, conn: &PgConnection) -> QueryResult<Integration> {
        use web::schema::integrations::dsl::*;

        integrations
            .filter(id.eq(integration_id))
            .get_result::<Integration>(conn)
    }

    pub fn delete(&self, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::delete;

        delete(self).execute(conn)
    }
}

#[derive(Insertable, Debug)]
#[table_name = "integrations"]
pub struct NewIntegration<'a> {
    pub user_id: i32,
    pub provider: &'a str,
    pub access_token: &'a str,
}

impl<'a> NewIntegration<'a> {
    pub fn new(user: &User, provider: &'a str, access_token: &'a str) -> Self {
        NewIntegration {
            user_id: user.id,
            provider: provider,
            access_token: access_token,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Integration> {
        use diesel::insert_into;

        insert_into(integrations::table)
            .values(self)
            .get_result::<Integration>(conn)
    }
}

#[derive(Identifiable, Queryable, Associations, Debug, Serialize)]
#[belongs_to(User)]
pub struct Device {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub kind: String,
    pub identifier: String,
}

impl Device {
    pub fn by_id(&self, device_id: i32, conn: &PgConnection) -> QueryResult<Device> {
        use web::schema::devices::dsl::*;

        devices
            .filter(id.eq(device_id))
            .get_result::<Device>(conn)
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
