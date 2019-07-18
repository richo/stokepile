use diesel::prelude::*;

use crate::web::schema::invites;

use chrono;
use rocket::http::RawStr;
use rocket::request::FromFormValue;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct Invite {
    pub id: i32,
    pub email: String,
    consumed: Option<chrono::naive::NaiveDateTime>,
}

impl Invite {
    pub fn by_email(conn: &PgConnection, email: &str) -> Result<Invite, diesel::result::Error> {
        use crate::web::schema::invites::dsl::{email as invite_email, invites};

        invites.filter(invite_email.eq(email)).get_result::<Invite>(conn)
    }

    pub fn consume(&self, conn: &PgConnection) -> QueryResult<usize> {
        use diesel::update;
        use crate::web::schema::invites::dsl::*;

        let now = chrono::Utc::now().naive_utc();
        update(self)
            .set((
                    consumed.eq(Some(now)),
                    ))
            .execute(conn)
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed.is_some()
    }
}

#[derive(Insertable, Debug)]
#[table_name = "invites"]
pub struct NewInvite<'a> {
    pub email: &'a str,
    consumed: Option<chrono::naive::NaiveDateTime>,
}

impl<'a> NewInvite<'a> {
    pub fn new(email: &'a str) -> Self {
        // TODO(richo) Assert this user doesn't already have an account or an invite

        NewInvite {
            email: email,
            consumed: None,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Invite> {
        use diesel::insert_into;

        insert_into(invites::table)
            .values(self)
            .get_result::<Invite>(conn)
    }
}
