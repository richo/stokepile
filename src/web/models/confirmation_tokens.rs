use std::iter;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use rocket::http::RawStr;

use super::*;
use diesel::prelude::*;

use crate::web::schema::confirmation_tokens;

const TOKEN_LENGTH: usize = 32;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct ConfirmationToken {
    pub id: i32,
    pub user_id: i32,
    #[serde(skip_serializing)]
    pub token: String,
}

impl ConfirmationToken {
    pub fn regenerate_token(&self, conn: &PgConnection) -> QueryResult<ConfirmationToken> {
        use diesel::update;
        use crate::web::schema::confirmation_tokens::dsl::*;

        update(self)
            .set(
                token.eq(random_token(TOKEN_LENGTH)),
            )
            .get_result(conn)
    }
}

impl PartialEq<RawStr> for ConfirmationToken {
    // TODO(richo) constant time compare
    fn eq(&self, other: &RawStr) -> bool {
        self.token.as_str() == other.as_str()
    }
}

#[derive(Insertable, Debug)]
#[table_name = "confirmation_tokens"]
pub struct NewConfirmationToken {
    pub user_id: i32,
    pub token: String,
}

fn random_token(length: usize) -> String {
    let mut rng = thread_rng();
    iter::repeat(())
        .map(|_| rng.sample(Alphanumeric))
        .take(length)
        .collect()
}

impl NewConfirmationToken {
    pub fn new(user: &User) -> Self {
        let token = random_token(TOKEN_LENGTH);
        NewConfirmationToken {
            user_id: user.id,
            token: token,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<ConfirmationToken> {
        use diesel::insert_into;

        insert_into(confirmation_tokens::table)
            .values(self)
            .get_result::<ConfirmationToken>(conn)
    }
}
