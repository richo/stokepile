use diesel::prelude::*;

use crate::web::schema::components;
use serde_json;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct Component {
    pub id: i32,
    pub equipment_id: i32,
    pub kind: String,
    pub model: String,
    pub serial: String,
    pub manufactured: chrono::naive::NaiveDateTime,

    pub data: serde_json::Value,
}


#[derive(Insertable, Debug)]
#[table_name = "components"]
pub struct NewComponent<'a> {
    pub equipment_id: i32,
    pub kind: &'a str,
    pub model: &'a str,
    pub serial: &'a str,
    pub manufactured: &'a chrono::naive::NaiveDateTime,

    pub data: &'a serde_json::Value,
}
