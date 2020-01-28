use diesel::prelude::*;

use crate::web::schema::components;
use serde_json;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct Component {
    pub id: i32,
    pub equipment_id: i32,
    pub kind: String,
    pub manufacturer: String,
    pub model: String,
    pub serial: String,
    pub manufactured: chrono::naive::NaiveDate,

    pub data: serde_json::Value,
}


#[derive(Insertable, Debug)]
#[table_name = "components"]
pub struct NewComponent<'a> {
    // we create this as 0 but assert that it's nonzero before we can actually create it.
    pub equipment_id: i32,
    pub kind: &'a str,
    pub manufacturer: &'a str,
    pub model: &'a str,
    pub serial: &'a str,
    pub manufactured: &'a chrono::naive::NaiveDate,
}

impl<'a> NewComponent<'a> {
    pub fn create(mut self, equipment_id: i32, conn: &PgConnection) -> QueryResult<Component> {
        self.equipment_id = equipment_id;

        use diesel::insert_into;

        insert_into(components::table)
            .values(self)
            .get_result::<Component>(conn)
    }
}
