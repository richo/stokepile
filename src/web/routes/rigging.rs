use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::{Context, PossibleIntegration};
use crate::web::models::{Customer, NewCustomer};

use rocket::request::FlashMessage;
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
    let customers = Customer::all(&*conn).expect("Couldn't load customers");
    let view_data = CustomerView { customers };
    let context = Context::rigging(view_data)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/customers", context)
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
    equipment: Vec<()>,
}

#[get("/equipment")]
pub fn equipment(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let equipment: EquipmentView = unimplemented!();
    let context = Context::rigging(equipment)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/equipment", context)
}
