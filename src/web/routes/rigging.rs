use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::{Context, PossibleIntegration};
use crate::web::models::{Customer, NewCustomer, Equipment, NewEquipment};

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
    let customers = Customer::all(&*conn, user.id()).expect("Couldn't load customers");

    let view_data = CustomerView { customers };
    let context = Context::rigging(view_data)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/customers", context)
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
}

#[derive(FromForm, Debug, Serialize)]
pub struct NewEquipmentForm {
    pub container: String,
    pub reserve: String,
    pub aad: String,
}

#[get("/equipment?<customer_id>")]
pub fn equipment(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>, customer_id: Option<i32>) -> Template {
    let list = match customer_id {
        Some(id) => {
            Equipment::for_customer(&*conn, id)
        },
        None => {
            Equipment::all(&*conn)
        }
    };

    let equipment = EquipmentView {
        equipment: list.expect("Couldn't load equipment"),
    };

    let context = Context::rigging(equipment)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/equipment", context)
}
