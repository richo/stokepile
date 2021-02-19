use crate::web::links;
use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::Context;
use crate::web::models::{
    Assembly,
    Customer, NewCustomer,
    Equipment, NewCompleteEquipment,
    Repack, NewRepack,
    User,
};
use crate::web::forms::rigging::{
    NewEquipmentForm,
    NewCustomerForm,
    RepackForm,
};

use rocket::request::{Form, FlashMessage};
use rocket::response::{status, Flash, Redirect};
use rocket_contrib::templates::Template;

use chrono::prelude::*;

pub static EQUIPMENT_KINDS: &[&'static str] = &["container", "reserve", "aad"];

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
}

#[get("/customer/<id>")]
pub fn customer_detail(user: WebUser, conn: DbConn, id: i32, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let customer = user.user.customer_by_id(&*conn, id).expect("Couldn't load customers");
    let equipment = customer.equipment(&*conn).expect("Couldn't load equipment");

    let view_data = CustomerDetailView {
        customer,
        equipment,
    };

    let context = Context::rigging(view_data)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/customer-detail", context)
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
    equipment: Vec<Assembly>,
    customer: Option<Customer>,
    equipment_kinds: &'static [&'static str],
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
        equipment_kinds: EQUIPMENT_KINDS,
    };

    let context = Context::rigging(equipment)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/equipment", context)
}

// TODO(richo) Is this just a param on the /equipment endpoint?
#[get("/equipment/due")]
pub fn due_equipment(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>) -> Template {
    use chrono::Duration;

    let list = get_equipment(&conn, &user.user, None);
    let filter_date = Utc::now()
        .naive_utc()
        .checked_add_signed(Duration::days(30))
        .expect("30 days from now")
        .date();

    let list = list.into_iter().filter(|asm| asm.due_before(filter_date));

    let equipment = EquipmentView {
        equipment: list.collect(),
        customer: None,
        equipment_kinds: EQUIPMENT_KINDS,
    };

    let context = Context::rigging(equipment)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/equipment", context)
}

fn get_equipment(conn: &DbConn, user: &User, customer_id: Option<i32>) -> Vec<Assembly> {
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
    .into_iter()
    .map(|equipment| { equipment.to_assembly(&*conn).expect("Couldn't load assembly") })
    .collect()
}

#[derive(Debug, Serialize)]
struct EquipmentDetailView {
    equipment: Assembly,
    repacks: Vec<Repack>,
    today: String,
}

pub fn today_for_form() -> String {
    // TODO(richo) timezones? Do we do this in JS?

    Utc::today()
        .format("%Y-%m-%d")
        .to_string()
}

#[get("/equipment/<equipment_id>")]
pub fn equipment_detail(user: WebUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>, equipment_id: i32) -> Template {
    let equipment = Equipment::by_user_and_id(&user, equipment_id, &*conn)
        .expect("Couldn't load equipment")
        .to_assembly(&*conn)
        .expect("Couldn't convert to assembly");
    let repacks = equipment.repacks(&*conn)
        .expect("Couldn't load repacks");

    let view = EquipmentDetailView {
        equipment,
        repacks,
        today: today_for_form(),
    };

    let context = Context::rigging(view)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("rigging/equipment-detail", context)
}

#[derive(Debug, Serialize)]
pub enum MissingRecordKind {
    Customer,
    Equipment,
}

#[derive(Debug, Serialize)]
pub struct ErrorContext {
    kind: MissingRecordKind,
    id: i32,
}

#[post("/customer/<customer_id>/equipment", data = "<equipment>")]
pub fn equipment_create(user: WebUser,
                        conn: DbConn,
                        flash: Option<FlashMessage<'_, '_>>,
                        equipment: Form<NewEquipmentForm>,
                        customer_id: i32) -> Result<Redirect, status::NotFound<Template>> {
    let customer = match user.user.customer_by_id(&*conn, customer_id) {
        Ok(customer) => customer,
        Err(not_found) => {
            let error = ErrorContext {
                kind: MissingRecordKind::Customer,
                id: customer_id,
            };
            let context = Context::error(error);
            return Err(status::NotFound(Template::render("rigging/not-found", context)))
        }
    };

    let assembly = NewCompleteEquipment::from(&equipment, &customer, &user.user);
    assembly.create(&*conn).expect("Couldn't create new equipment");

    Ok(Redirect::to(links::equipment_link_for_customer(customer_id.into())))
}

#[post("/equipment/<equipment_id>/repack", data = "<repack>")]
pub fn repack_create(user: WebUser,
                        conn: DbConn,
                        flash: Option<FlashMessage<'_, '_>>,
                        repack: Form<RepackForm>,
                        equipment_id: i32) -> Result<Redirect, status::NotFound<Template>> {
    let equipment = match user.user.equipment_by_id(&*conn, equipment_id) {
        Ok(equipment) => equipment,
        Err(not_found) => {
            let error = ErrorContext {
                kind: MissingRecordKind::Equipment,
                id: equipment_id,
            };
            let context = Context::error(error);
            // TODO(richo) Load this onto the ErrorContext?
            // If all the View structs have their renderers attached it should be easier to assert
            // that they're correct
            return Err(status::NotFound(Template::render("rigging/not-found", context)))
        }
    };

    let record = NewRepack::from_form(&user.user, equipment_id, &repack);
    record.create(&*conn).expect("Couldn't create record");

    Ok(Redirect::to(links::equipment_detail_link(equipment_id.into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;
    use crate::web::test_helpers::rigging::*;

    use rocket::http::{ContentType, Status};

    client_for_routes!(equipment_create => client);

    #[test]
    fn test_create_equipment_with_aad() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let form = "\
container_manufacturer=c_mfgr&\
container_model=c_mdl&\
container_serial=c_srl&\
container_dom=2010-12-30&\
reserve_manufacturer=r_mfgr&\
reserve_model=r_mdl&\
reserve_serial=r_srl&\
reserve_dom=2011-11-29&\
aad_manufacturer=a_mfgr&\
aad_model=a_mdl&\
aad_serial=a_srl&\
aad_dom=2012-10-28&\
";

        let conn = db_conn(&client);
        let customer = create_customer(&user, &*conn);
        drop(conn);

        let req = client
            .post(format!("/customer/{}/equipment", customer.id))
            .header(ContentType::Form)
            .body(form);

        let response = req.dispatch();

        assert_eq!(response.status(), Status::SeeOther);

        let conn = db_conn(&client);
        let mut equipment = customer.equipment(&*conn).expect("Couldn't load equipment");
        assert_eq!(1, equipment.len());

        let assembly = equipment.pop()
            .unwrap()
            .to_assembly(&*conn)
            .expect("Couldn't load assembly");
        assert_eq!(3, assembly.components.len());

        let container = assembly.container();
        assert_eq!(container.manufacturer, "c_mfgr");
        assert_eq!(container.model, "c_mdl");
        assert_eq!(container.serial, "c_srl");
        assert_eq!(container.manufactured, NaiveDate::from_ymd(2010, 12, 30));

        let reserve = assembly.reserve();
        assert_eq!(reserve.manufacturer, "r_mfgr");
        assert_eq!(reserve.model, "r_mdl");
        assert_eq!(reserve.serial, "r_srl");
        assert_eq!(reserve.manufactured, NaiveDate::from_ymd(2011, 11, 29));

        let aad = assembly.aad().unwrap();
        assert_eq!(aad.manufacturer, "a_mfgr");
        assert_eq!(aad.model, "a_mdl");
        assert_eq!(aad.serial, "a_srl");
        assert_eq!(aad.manufactured, NaiveDate::from_ymd(2012, 10, 28));
    }

    #[test]
    fn test_create_equipment_without_aad() {
        init_env();

        let client = client();
        let user = create_user(&client, "test@email.com", "p@55w0rd");
        signin(&client, "test%40email.com", "p%4055w0rd").unwrap();

        let form = "\
container_manufacturer=c_mfgr&\
container_model=c_mdl&\
container_serial=c_srl&\
container_dom=2010-12-30&\
reserve_manufacturer=r_mfgr&\
reserve_model=r_mdl&\
reserve_serial=r_srl&\
reserve_dom=2011-11-29&\
aad_manufacturer=&\
aad_model=&\
aad_serial=&\
aad_dom=\
";

        let conn = db_conn(&client);
        let customer = create_customer(&user, &*conn);
        drop(conn);

        let req = client
            .post(format!("/customer/{}/equipment", customer.id))
            .header(ContentType::Form)
            .body(form);

        let response = req.dispatch();

        assert_eq!(response.status(), Status::SeeOther);

        let conn = db_conn(&client);
        let mut equipment = customer.equipment(&*conn).expect("Couldn't load equipment");
        assert_eq!(1, equipment.len());

        let assembly = equipment.pop()
            .unwrap()
            .to_assembly(&*conn)
            .expect("Couldn't load assembly");
        assert_eq!(2, assembly.components.len());

        let container = assembly.container();
        assert_eq!(container.manufacturer, "c_mfgr");
        assert_eq!(container.model, "c_mdl");
        assert_eq!(container.serial, "c_srl");
        assert_eq!(container.manufactured, NaiveDate::from_ymd(2010, 12, 30));

        let reserve = assembly.reserve();
        assert_eq!(reserve.manufacturer, "r_mfgr");
        assert_eq!(reserve.model, "r_mdl");
        assert_eq!(reserve.serial, "r_srl");
        assert_eq!(reserve.manufactured, NaiveDate::from_ymd(2011, 11, 29));

        let aad = assembly.aad();
        assert!(aad.is_none());
    }
}
