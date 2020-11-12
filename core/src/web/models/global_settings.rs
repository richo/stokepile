use diesel::prelude::*;
use crate::web::schema::global_settings;

#[derive(Identifiable, Queryable, Debug, Serialize)]
#[primary_key(onerow_id)]
pub struct GlobalSetting {
    onerow_id: bool,
    invites_required: bool,
}

impl GlobalSetting {
    pub fn get(conn: &PgConnection,) -> QueryResult<GlobalSetting> {
        use crate::web::schema::global_settings::dsl::*;

        global_settings.first(conn)
    }

    pub fn invites_required(&self) -> bool {
        self.invites_required
    }
}
