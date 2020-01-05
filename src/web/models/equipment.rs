use diesel::prelude::*;

use crate::web::schema::equipment;
use crate::web::routes::rigging::NewEquipmentForm;
use crate::web::models::{User, Customer};

#[derive(Identifiable, Queryable, Debug, Serialize)]
#[table_name = "equipment"]
pub struct Equipment {
    pub id: i32,
    pub user_id: i32,
    pub customer_id: i32,
}


#[derive(Insertable, Debug)]
#[table_name = "equipment"]
pub struct NewEquipment {
    pub user_id: i32,
    pub customer_id: i32,
}

impl Equipment {
    // TODO(richo) paginate
    pub fn all(conn: &PgConnection) -> QueryResult<Vec<Equipment>> {
        use crate::web::schema::equipment::dsl::*;

        equipment.load::<Equipment>(&*conn)
    }
}

// Should htis actually just be a Equipment::create ?
impl NewEquipment {
    // Do we want some request global user_id? Seems positive but I also don't really see how to
    // make it happen.
    pub fn from(equipment: &NewEquipmentForm, customer: &Customer, user: &User) -> NewEquipment {
        NewEquipment {
            user_id: user.id,
            customer_id: customer.id,
        }

        // TODO(richo) populate the components
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Equipment> {
        use diesel::insert_into;

        insert_into(equipment::table)
            .values(self)
            .get_result::<Equipment>(conn)
    }
}
