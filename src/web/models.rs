#![allow(proc_macro_derive_resolution_fallback)]

use serde_json;

use bcrypt;
use diesel::prelude::*;
use rand;

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

#[derive(Identifiable, Queryable, Associations, Debug)]
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
