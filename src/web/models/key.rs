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
}

#[derive(Insertable, Debug)]
#[table_name = "keys"]
pub struct NewKey {
    pub id: String,
    pub user_id: i32,
    pub token: String,
    created: chrono::naive::NaiveDateTime,
    expired: Option<chrono::naive::NaiveDateTime>,
}

fn generate_token_id() -> String {
    let (x, y) = rand::random::<(u64, u64)>();
    format!("{:x}{:x}", x, y)
}

impl NewKey {
    pub fn new(user: &User) -> Self {
        NewKey {
            id: generate_token_id(),
            user_id: user.id,
            token: generate_token_id(),
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
