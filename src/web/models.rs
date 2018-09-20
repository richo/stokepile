#![allow(proc_macro_derive_resolution_fallback)]
use bcrypt;
use diesel::prelude::*;

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

        let user = users
            .filter(user_email.eq(email))
            .get_result::<User>(conn)
            .unwrap();

        if bcrypt::verify(password, &user.password).unwrap() {
            Some(user)
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
