use diesel::prelude::*;
use chrono;

use web::schema::keys;
use super::*;

#[derive(Identifiable, Queryable, Associations, Debug, AsChangeset, PartialEq, Serialize)]
#[belongs_to(User)]
pub struct Key {
    pub id: i32,
    pub user_id: i32,
    pub token: String,
    pub created: chrono::naive::NaiveDateTime,
    pub expired: Option<chrono::naive::NaiveDateTime>,
}

impl Key {
    pub fn by_token(conn: &PgConnection, token_id: &str) -> QueryResult<(Self, User)> {
        use web::schema::keys::dsl::*;
        use web::schema::users;

        keys
            .inner_join(users::table)
            .filter(token.eq(token_id))
            .get_result::<(Key, User)>(conn)
    }

    pub fn expire(&self, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::update;
        use web::schema::keys::dsl::*;

        let now = chrono::Utc::now().naive_utc();

        update(self).set(expired.eq(now)).execute(conn)
    }

    pub fn is_expired(&self) -> bool {
        match self.expired {
            Some(ts) => {
                let now = chrono::Utc::now().naive_utc();
                ts < now
            },
            None => false,
        }
    }
}

#[derive(Insertable, Debug)]
#[table_name = "keys"]
pub struct NewKey {
    pub user_id: i32,
    pub token: String,
    created: chrono::naive::NaiveDateTime,
    expired: Option<chrono::naive::NaiveDateTime>,
}

impl NewKey {
    pub fn new(user: &User) -> Self {
        NewKey {
            user_id: user.id,
            token: generate_secret(),
            created: chrono::Utc::now().naive_utc(),
            expired: None,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Key> {
        use diesel::insert_into;

        insert_into(keys::table)
            .values(self)
            .get_result::<Key>(conn)
    }
}
