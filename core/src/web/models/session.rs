use diesel::prelude::*;
use serde_json;

use super::*;
use crate::web::schema::sessions;

#[derive(Identifiable, Queryable, Associations, Debug, AsChangeset, PartialEq)]
#[belongs_to(User)]
pub struct Session {
    pub id: String,
    pub user_id: i32,
    pub data: serde_json::Value,
}

impl Session {
    pub fn by_id(conn: &PgConnection, session_id: &str) -> QueryResult<(Self, User)> {
        use crate::web::schema::sessions::dsl::*;
        use crate::web::schema::users;

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
        use crate::web::schema::sessions::dsl::*;

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

impl NewSession {
    pub fn new(user: &User) -> Self {
        NewSession {
            id: generate_secret(),
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
