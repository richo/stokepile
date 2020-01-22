use diesel::prelude::*;

use crate::web::schema::equipment;
use crate::web::routes::rigging::NewEquipmentForm;
use crate::web::models::{User, Customer, Component, NewComponent};

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
    user_id: i32,
    customer_id: i32,
}

impl Equipment {
    // // TODO(richo) paginate
    // pub fn all(conn: &PgConnection) -> QueryResult<Vec<Equipment>> {
    //     use crate::web::schema::equipment::dsl::*;

    //     equipment.load::<Equipment>(&*conn)
    // }

    // TODO(richo)
    // pub fn components(conn: &PgConnection) -> QueryResult<Vec<Component>> {
    // }
}

// Should htis actually just be a Equipment::create ?
impl NewEquipment {

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Equipment> {
        use diesel::insert_into;

        insert_into(equipment::table)
            .values(self)
            .get_result::<Equipment>(conn)
    }
}

/// A container struct representing the global equipment object as well as it's associated
/// components.
#[derive(Debug)]
pub struct NewCompleteEquipment<'a> {
    equipment: NewEquipment,
    components: Vec<NewComponent<'a>>,
}

impl<'a> NewCompleteEquipment<'a> {
    // Do we want some request global user_id? Seems positive but I also don't really see how to
    // make it happen.
    pub fn from(equipment: &NewEquipmentForm, customer: &Customer, user: &User) -> Self {
        let components = vec![];
        // TODO(richo) populate the components

        NewCompleteEquipment {
            equipment: NewEquipment {
                user_id: user.id,
                customer_id: customer.id,
            },
            components,
        }
    }

    pub fn create(self, conn: &PgConnection) -> QueryResult<Equipment> {
        conn.transaction(|| {
            let equipment = self.equipment.create(&conn)?;

            for component in self.components {
                component.create(equipment.id, &conn)?;
            }

            Ok(equipment)
        })
    }
}
