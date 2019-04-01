use chrono::prelude::*;
use diesel::prelude::*;

use super::*;
use crate::web::schema::integrations;

#[derive(Identifiable, Queryable, Associations, Debug)]
#[belongs_to(User)]
pub struct Integration {
    pub id: i32,
    pub user_id: i32,
    pub provider: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub refreshed: chrono::naive::NaiveDateTime,
}

impl Integration {
    pub fn by_id(&self, integration_id: i32, conn: &PgConnection) -> QueryResult<Integration> {
        use crate::web::schema::integrations::dsl::*;

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
    pub refresh_token: Option<&'a str>,
    pub refreshed: chrono::naive::NaiveDateTime,
}

impl<'a> NewIntegration<'a> {
    pub fn new(user: &User, provider: &'a str, access_token: &'a str, refresh_token: Option<&'a str>) -> Self {
        NewIntegration {
            user_id: user.id,
            provider,
            access_token,
            refresh_token,
            refreshed: Utc::now().naive_utc(),
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Integration> {
        use diesel::insert_into;

        insert_into(integrations::table)
            .values(self)
            .get_result::<Integration>(conn)
    }
}
