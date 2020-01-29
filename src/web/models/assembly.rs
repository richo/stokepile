use diesel::prelude::*;
use chrono::NaiveDate;

use crate::web::models::{Equipment, Component, Repack};

#[derive(Debug, Serialize)]
pub struct Assembly {
    pub equipment: Equipment,
    pub components: Vec<Component>,
    pub last_repack: Option<Repack>,
    pub next_due: Option<NaiveDate>,
}

impl Assembly {
    pub fn repacks(&self, conn: &PgConnection) -> QueryResult<Vec<Repack>> {
        Repack::by_equipment(self.equipment.id, conn)
    }
}
