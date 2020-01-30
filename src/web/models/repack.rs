use diesel::prelude::*;
use chrono::{Duration, NaiveDate};

use crate::web::schema::repacks;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct Repack {
    pub id: i32,
    rigger: i32,
    equipment: i32,
    pub date: NaiveDate,
    service: String,
    location: String,
}

impl Repack {
    // TODO(richo) Should this have some user bound?
    pub fn by_equipment(equipment_id: i32, conn: &PgConnection) -> QueryResult<Vec<Repack>> {
        use crate::web::schema::repacks::dsl::*;

        repacks
            .filter(equipment.eq(equipment_id))
            .order(date.asc())
            .load::<Repack>(conn)
    }

    // TODO(richo) Different countries?
    pub fn next_due(&self) -> NaiveDate {
        self.date.checked_add_signed(Duration::days(180)).unwrap()
    }
}

#[derive(Insertable, Debug)]
#[table_name = "repacks"]
pub struct NewRepack {
    rigger: i32,
    equipment: i32,
    date: NaiveDate,
}
