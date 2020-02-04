use diesel::prelude::*;

use crate::web::schema::equipment;
use crate::web::forms::rigging::NewEquipmentForm;
use crate::web::models::{Assembly, User, Customer, Component, NewComponent, Repack};

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

    pub fn components(&self, conn: &PgConnection) -> QueryResult<Vec<Component>> {
        use crate::web::schema::components::dsl::*;

        components.filter(equipment_id.eq(self.id)).load::<Component>(conn)
    }

    pub fn to_assembly(self, conn: &PgConnection) -> QueryResult<Assembly> {
        let components = self.components(&*conn)?;
        let last_repack = Repack::by_equipment(self.id, &*conn)?.pop();
        let next_due = last_repack.as_ref().map(|repack| repack.next_due());

        Ok(Assembly {
            equipment: self,
            components,
            last_repack,
            next_due,
        })
    }

    pub fn by_user_and_id(user: &User, equipment_id: i32, conn: &PgConnection) -> QueryResult<Equipment> {
        use crate::web::schema::equipment::dsl::*;
        equipment
            .filter(id.eq(equipment_id))
            .filter(user_id.eq(user.id))
            .get_result::<Equipment>(conn)
    }
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
pub struct NewCompleteEquipment<'b> {
    equipment: NewEquipment,
    components: Vec<NewComponent<'b>>,
}

impl<'a> NewCompleteEquipment<'a> {
    // Do we want some request global user_id? Seems positive but I also don't really see how to
    // make it happen.
    pub fn from(equipment: &'a NewEquipmentForm, customer: &Customer, user: &User) -> Self {
        let mut components = vec![];
        components.push(NewComponent {
            // This is filled in by `create`
            equipment_id: 0,
            kind: "container",
            manufacturer: &equipment.container.manufacturer,
            model: &equipment.container.model,
            serial: &equipment.container.serial,
            manufactured: &equipment.container.dom,
        });
        components.push(NewComponent {
            // This is filled in by `create`
            equipment_id: 0,
            kind: "reserve",
            manufacturer: &equipment.reserve.manufacturer,
            model: &equipment.reserve.model,
            serial: &equipment.reserve.serial,
            manufactured: &equipment.reserve.dom,
        });
        if let Some(ref aad) = equipment.aad {
            components.push(NewComponent {
                // This is filled in by `create`
                equipment_id: 0,
                kind: "aad",
                manufacturer: &aad.manufacturer,
                model: &aad.model,
                serial: &aad.serial,
                manufactured: &aad.dom,
            });
        }

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

            for mut component in self.components {
                // First set the equipment_id, then create.
                component.equipment_id = equipment.id;
                component.create(equipment.id, &conn)?;
            }

            Ok(equipment)
        })
    }
}
