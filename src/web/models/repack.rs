use diesel::prelude::*;
use chrono::{Duration, NaiveDate};

use crate::web::schema::repacks;
use crate::web::models::User;

use crate::web::forms::rigging::RepackForm;


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
pub struct NewRepack<'a> {
    rigger: i32,
    equipment: i32,
    date: NaiveDate,
    service: &'a str,
    location: &'a str,
}

impl<'a> NewRepack<'a> {
    pub fn create(&self, conn: &PgConnection) -> QueryResult<Repack> {
        use diesel::insert_into;

        insert_into(repacks::table)
            .values(self)
            .get_result::<Repack>(conn)
    }

    pub fn from_form(rigger: &User, equipment: i32, form: &'a RepackForm) -> NewRepack<'a> {
        NewRepack {
            rigger: rigger.id,
            equipment,
            date: *form.date,
            service: &form.service,
            location: &form.place,
        }
    }
}
