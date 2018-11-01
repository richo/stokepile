use serde_json;
use rand;

use diesel::prelude::*;
use chrono;

use web::schema::keys;
use super::*;

#[derive(Identifiable, Queryable, Associations, Debug, AsChangeset, PartialEq)]
#[belongs_to(User)]
pub struct Key {
    pub id: String,
    pub user_id: i32,
    pub token: String,
    pub expired: Option<chrono::naive::NaiveDateTime>,
}

impl Key {
    pub fn by_id(conn: &PgConnection, session_id: &str) -> QueryResult<(Self, User)> {
        use web::schema::sessions::dsl::*;
        use web::schema::keys;

        sessions
            .inner_join(users::table)
            .filter(id.eq(session_id))
            .get_result::<(Key, User)>(conn)
    }

    pub fn expire(&self, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::update;
        use web::schema::sessions::dsl::*;

        let now = chono::Utc::now().naive_utc();

        update(self).set(expired.eq(now)).execute(conn)
    }
}

#[derive(Insertable, Debug)]
#[table_name = "keys"]
pub struct NewKey {
    pub id: String,
    pub user_id: i32,
    pub token: String,
}

fn generate_token_id() -> String {
    let (x, y) = rand::random::<(u64, u64)>();
    format!("{:x}{:x}", x, y)
}

impl NewKey {
    pub fn new(user: &User) -> Self {
        NewSession {
            id: generate_token_id(),
            user_id: user.id,
            token: generate_token_id(),
            expired: None,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Session> {
        use diesel::insert_into;

        insert_into(sessions::table)
            .values(self)
            .get_result::<Session>(conn)
    }
}
