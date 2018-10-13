use diesel::prelude::*;

use web::schema::integrations;
use super::*;

#[derive(Identifiable, Queryable, Associations, Debug)]
#[belongs_to(User)]
pub struct Integration {
    pub id: i32,
    pub user_id: i32,
    pub provider: String,
    pub access_token: String,
}

impl Integration {
    pub fn by_id(&self, integration_id: i32, conn: &PgConnection) -> QueryResult<Integration> {
        use web::schema::integrations::dsl::*;

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
