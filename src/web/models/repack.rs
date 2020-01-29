use diesel::prelude::*;
use chrono::NaiveDate;

use crate::web::schema::repacks;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct Repack {
    pub id: i32,
    rigger: i32,
    equipment: i32,
    date: NaiveDate,
}

impl Repack {
    // TODO(richo) Should this have some user bound?
    pub fn by_equipment(id: i32, conn: &PgConnection) -> QueryResult<Vec<Repack>> {
        use crate::web::schema::repacks::dsl::*;

        repacks
            .filter(equipment.eq(id))
            .order(date.asc())
            .load::<Repack>(conn)
    }
}

#[derive(Insertable, Debug)]
#[table_name = "repacks"]
pub struct NewRepack {
    rigger: i32,
    equipment: i32,
    date: NaiveDate,
}
