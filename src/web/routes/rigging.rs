use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::Context;
use crate::web::models::{Customer, NewCustomer, Equipment, NewEquipment, User};

use rocket::request::{Form, FlashMessage};
use rocket::response::{Flash, Redirect};
use rocket_contrib::templates::Template;

#[get("/")]
pub fn index(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let context = Context::rigging(())
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/index", context)
}

#[derive(Debug, Serialize)]
struct CustomerView {
    customers: Vec<Customer>,
}

#[get("/customers")]
pub fn customers(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let customers = user.user.customers(&*conn).expect("Couldn't load customers");

    let view_data = CustomerView { customers };
    let context = Context::rigging(view_data)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/customers", context)
}

#[derive(Debug, Serialize)]
struct CustomerDetailView {
    customer: Customer,
    equipment: Vec<Equipment>,
    repacks: (),
}

#[get("/customer/<id>")]
pub fn customer_detail(user: WebUser, conn: DbConn, id: i32, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let customer = user.user.customer_by_id(&*conn, id).expect("Couldn't load customers");
    let equipment = customer.equipment(&*conn).expect("Couldn't load equipment");

    let view_data = CustomerDetailView {
        customer,
        equipment,
        repacks: (),
    };

    let context = Context::rigging(view_data)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/customer-detail", context)
}

#[derive(FromForm, Debug, Serialize)]
pub struct NewCustomerForm {
    pub name: String,
    pub address: String,
    pub phone_number: String,
    pub email: String,
}

#[post("/customers/create", data = "<customer>")]
pub fn customer_create(user: WebUser, conn: DbConn, customer: Form<NewCustomerForm>) -> Flash<Redirect> {
    match NewCustomer::from(&customer, user.id()).create(&*conn) {
        Ok(_) => {
            Flash::success(
                Redirect::to("/rigging/customers"),
                format!("Successfully created customer"),
                )
        },
        Err(e) => {
            Flash::error(
                Redirect::to("/rigging/customers"),
                format!("Error creating customer, {:?}", e),
                )
        }
    }
}

#[derive(Debug, Serialize)]
struct ServiceBulletinView {
    service_bulletins: Vec<()>,
}

#[get("/service_bulletins")]
pub fn service_bulletins(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let service_bulletins: ServiceBulletinView = unimplemented!();
    let context = Context::rigging(service_bulletins)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/service_bulletins", context)
}

#[derive(Debug, Serialize)]
struct EquipmentView {
    equipment: Vec<Equipment>,
    customer: Option<Customer>,
}

#[derive(FromForm, Debug, Serialize)]
pub struct NewEquipmentForm {
    pub customer_id: i32,
    pub container: String,
    pub reserve: String,
    pub aad: String,
}

#[get("/equipment?<customer_id>")]
pub fn equipment(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>, customer_id: Option<i32>) -> Template {
    // TODO(This doesn't validate that the customer belongs to this user at all)
    let list = get_equipment(&conn, &user.user, customer_id);
    // TODO(richo) This absolutely does something bad in the face of an invalid ID
    let customer = customer_id.map(|id| user.user.customer_by_id(&*conn, id).expect("Couldn't load customer"));

    let equipment = EquipmentView {
        equipment: list,
        customer,
    };

    let context = Context::rigging(equipment)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/equipment", context)
}

fn get_equipment(conn: &DbConn, user: &User, customer_id: Option<i32>) -> Vec<Equipment> {
    match customer_id {
        Some(id) => {
            user.customer_by_id(&*conn, id)
                // TODO(richo) This is actually an urgently pressing case where we need to figure
                // out how to present errors to the user.
                .expect("Couldn't load customer")
                // TODO(richo) Do we care about figuring out how to avoid the N+1 query here?
                .equipment(&*conn)
                .expect("Couldn't load equipment for customer")
        },
        None => {
            user.equipment(&*conn)
                .expect("Couldn't load equipment for customer")
        }
    }
}

#[post("/equipment/create", data = "<equipment>")]
pub fn equipment_create(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>, equipment: Form<NewEquipmentForm>) -> Template {
    let customer_id = equipment.customer_id;
    let customer = user.user.customer_by_id(&*conn, customer_id).expect("Couldn't load customer");
    let equipment = NewEquipment::from(&equipment, &customer, &user.user);
    equipment.create(&*conn).expect("Couldn't create new equipment");

    let list = get_equipment(&conn, &user.user, Some(customer_id));

    let equipment = EquipmentView {
        equipment: list,
        customer: Some(customer),
    };

    let context = Context::rigging(equipment)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/equipment", context)
}
