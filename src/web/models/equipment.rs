use diesel::prelude::*;

use crate::web::schema::equipment;
use crate::web::routes::rigging::NewEquipmentForm;

#[derive(Identifiable, Queryable, Debug, Serialize)]
#[table_name = "equipment"]
pub struct Equipment {
    pub id: i32,
    pub user_id: i32,
    pub customer_id: i32,
    pub container: String,
    pub reserve: String,
    pub aad: String,
}


#[derive(Insertable, Debug)]
#[table_name = "equipment"]
pub struct NewEquipment<'a> {
    pub user_id: i32,
    pub customer_id: i32,
    pub container: &'a str,
    pub reserve: &'a str,
    pub aad: &'a str,
}

impl Equipment {
    // TODO(richo) paginate
    pub fn all(conn: &PgConnection) -> QueryResult<Vec<Equipment>> {
        use crate::web::schema::equipment::dsl::*;

        equipment.load::<Equipment>(&*conn)
    }
}

// Should htis actually just be a Equipment::create ?
impl<'a> NewEquipment<'a> {
    // Do we want some request global user_id? Seems positive but I also don't really see how to
    // make it happen.
    pub fn from(equipment: &'a NewEquipmentForm, customer_id: i32, user_id: i32) -> NewEquipment<'_> {
        NewEquipment {
            user_id,
            customer_id,
            container: &equipment.container,
            reserve: &equipment.reserve,
            aad: &equipment.aad,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Equipment> {
        use diesel::insert_into;

        insert_into(equipment::table)
            .values(self)
            .get_result::<Equipment>(conn)
    }
}
