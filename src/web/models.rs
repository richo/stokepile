#![allow(proc_macro_derive_resolution_fallback)]

use serde_json;

use bcrypt;
use diesel::prelude::*;
use rand;

use web::schema::sessions;
use web::schema::users;

#[derive(Queryable, Debug)]
pub struct User {
    pub id: i32,
    pub email: String,
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
    pub fn by_id(conn: &PgConnection, session_id: &str) -> QueryResult<Self> {
        use web::schema::sessions::dsl::*;

        sessions
            .filter(id.eq(session_id))
            .get_result::<Session>(conn)
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
