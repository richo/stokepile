use std::convert::{TryFrom, TryInto};

use crate::web::links;
use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::Context;
use crate::web::models::{Assembly, Customer, NewCustomer, Equipment, NewCompleteEquipment, User, Repack};

use rocket::http::RawStr;
use rocket::request::{Form, FromForm, FormItems, FlashMessage};
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
    equipment: Vec<Assembly>,
    customer: Option<Customer>,
    equipment_kinds: &'static [&'static str],
}

#[derive(Debug, Serialize)]
pub struct Component {
    pub manufacturer: String,
    pub model: String,
    pub serial: String,
    pub dom: NaiveDate,
}

#[derive(Debug, Default)]
struct ProtoComponent {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub dom: Option<NaiveDate>,
}

#[derive(Debug, Default)]
struct OptionalProtoComponent {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub dom: Option<NaiveDate>,
}

trait UnpackValue {
    fn convert_value(&self, value: &RawStr) -> Result<String, EquipmentFormError> {
        value.url_decode().map_err(|e| EquipmentFormError::Parsing(e))
    }

    fn convert_date(&self, value: &RawStr) -> Result<NaiveDate, EquipmentFormError> {
        NaiveDate::parse_from_str(&self.convert_value(value)?, "%Y-%m-%d")
            .map_err(|e| EquipmentFormError::DateParsing(e))
    }
}

impl UnpackValue for ProtoComponent {}
impl UnpackValue for OptionalProtoComponent {}

impl TryFrom<ProtoComponent> for Component {
    type Error = EquipmentFormError;

    fn try_from(value: ProtoComponent) -> Result<Self, Self::Error> {
        if let (Some(manufacturer), Some(model), Some(serial), Some(dom)) = (value.manufacturer, value.model, value.serial, value.dom) {
            Ok(Component {
                manufacturer,
                model,
                serial,
                dom,
            })
        } else {
            // TODO(richo) Good error
            Err(EquipmentFormError::MissingField)
        }
    }
}

impl TryFrom<OptionalProtoComponent> for Component {
    type Error = EquipmentFormError;

    fn try_from(value: OptionalProtoComponent) -> Result<Self, Self::Error> {
        if let (Some(manufacturer), Some(model), Some(serial), Some(dom)) = (value.manufacturer, value.model, value.serial, value.dom) {
            Ok(Component {
                manufacturer,
                model,
                serial,
                dom,
            })
        } else {
            // TODO(richo) Good error
            Err(EquipmentFormError::MissingField)
        }
    }
}

#[derive(Debug)]
pub enum EquipmentFormError {
    Parsing(std::str::Utf8Error),
    DateParsing(chrono::ParseError),
    MissingField,
    ExtraFields,
}

// TODO(richo) Support for an optional AAD
macro_rules! equipment_form_members {
    ( $item:expr, $( $name:ident => $struct:expr ),+ ) => (
        match $item.key.as_str() {
            // TODO(richo) Should we just stash the potentiaal errors in the proto object for a
            // while and barf later trying to convert?
            $(
                concat!(stringify!($name), "_manufacturer") => {
                    $struct.manufacturer = Some($struct.convert_value($item.value)?);
                },
                concat!(stringify!($name), "_model") => {
                    $struct.model = Some($struct.convert_value($item.value)?);
                },
                concat!(stringify!($name), "_serial") => {
                    $struct.serial = Some($struct.convert_value($item.value)?);
                },
                concat!(stringify!($name), "_dom") => {
                    $struct.dom = Some($struct.convert_date($item.value)?);
                },
            )+
                // TODO(richo)
            // _ if strict => return Err(EquipmentFormError::ExtraFields),
            field => { debug!("Got an extra field: {:?} => {:?}", &field, &$item.value); },
        }
    );
}

impl<'f> FromForm<'f> for NewEquipmentForm {
    // In practice, we'd use a more descriptive error type.
    type Error = EquipmentFormError;

    fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<NewEquipmentForm, EquipmentFormError> {
        let mut container = ProtoComponent::default();
        let mut reserve = ProtoComponent::default();
        let mut aad = OptionalProtoComponent::default();

        for item in items {
            equipment_form_members!(item,
                container => container,
                reserve => reserve,
                aad => aad);
        }

        Ok(NewEquipmentForm {
            container: container.try_into()?,
            reserve: reserve.try_into()?,
            // hahaaaaaa this is a stretch
            aad: aad.try_into().ok(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NewEquipmentForm {
    #[serde(flatten)]
    pub container: Component,

    #[serde(flatten)]
    pub reserve: Component,

    #[serde(flatten)]
    pub aad: Option<Component>,

    // TODO(richo) notes? Bury the notes in the data field?
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
pub struct ErrorContext {
    customer_id: i32,
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
            let error = ErrorContext { customer_id };
            let context = Context::error(error);
            return Err(status::NotFound(Template::render("rigging/customer_not_found", context)))
        }
    };

    let assembly = NewCompleteEquipment::from(&equipment, &customer, &user.user);
    assembly.create(&*conn).expect("Couldn't create new equipment");

    Ok(Redirect::to(links::equipment_link_for_customer(customer_id.into())))
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

        let aad = assembly.aad();
        assert!(aad.is_none());
    }
}
