use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::Context;
use crate::web::models::{Customer, NewCustomer, Equipment, NewCompleteEquipment, User};

use rocket::request::{Form, FromForm, FormItems, FlashMessage};
use rocket::response::{status, Flash, Redirect};
use rocket_contrib::templates::Template;

use chrono::naive::NaiveDate;

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
    equipment_kinds: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Component {
    pub model: String,
    pub serial: String,
    pub dom: NaiveDate,
}

#[derive(Debug, Default)]
struct ProtoComponent {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub dom: Option<NaiveDate>,
}

impl ProtoComponent {
    pub fn to_component(self) -> Result<Component, EquipmentFormError> {
        if let (Some(model), Some(serial), Some(dom)) = (self.model, self.serial, self.dom) {
            Ok(Component {
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
    ( $self:expr, $item:expr, $( $name:ident => $struct:expr ),+ ) => (
        match $item.key.as_str() {
            $(
                concat!(stringify!($name), "_model") => {
                    $struct.model = Some($item.value.url_decode().map_err(|e| EquipmentFormError::Parsing(e))?);
                },
                concat!(stringify!($name), "_serial") => {
                    $struct.serial = Some($item.value.url_decode().map_err(|e| EquipmentFormError::Parsing(e))?);
                },
                concat!(stringify!($name), "_dom") => {
                    $struct.dom = Some(NaiveDate::parse_from_str(&$item.value.url_decode().map_err(|e| EquipmentFormError::Parsing(e))?, "%Y-%m-%d")
                        .map_err(|e| EquipmentFormError::DateParsing(e))?);
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
        let mut aad = ProtoComponent::default();

        for item in items {
            equipment_form_members!(self, item,
                container => container,
                reserve => reserve,
                aad => aad);
                info!(stringify!(
            equipment_form_members!(self, item,
                container => container,
                reserve => reserve,
                aad => aad)
                ));
        }

        Ok(NewEquipmentForm {
            container: container.to_component()?,
            reserve: reserve.to_component()?,
            // hahaaaaaa this is a stretch
            aad: aad.to_component().ok(),
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
        equipment_kinds: vec!["container".into(), "reserve".into(), "aad".into()],
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

#[derive(Debug, Serialize)]
pub struct ErrorContext {
    customer_id: i32,
}

#[post("/customer/<customer_id>/equipment", data = "<equipment>")]
pub fn equipment_create(user: WebUser,
                        conn: DbConn,
                        flash: Option<FlashMessage<'_, '_>>,
                        equipment: Form<NewEquipmentForm>,
                        customer_id: i32) -> Result<Template, status::NotFound<Template>> {
    let customer = match user.user.customer_by_id(&*conn, customer_id) {
        Ok(customer) => customer,
        Err(not_found) => {
            let error = ErrorContext { customer_id };
            let context = Context::error(error);
            return Err(status::NotFound(Template::render("rigging/customer_not_found", context)))
        }
    };

    let equipment = NewCompleteEquipment::from(&equipment, &customer, &user.user);
    equipment.create(&*conn).expect("Couldn't create new equipment");

    let list = get_equipment(&conn, &user.user, Some(customer_id));

    let equipment = EquipmentView {
        equipment: list,
        customer: Some(customer),
        equipment_kinds: vec!["container".into(), "reserve".into(), "aad".into()],
    };

    let context = Context::rigging(equipment)
        .set_user(Some(user))
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    // This should actually be a redirect to the equipment detail view
    Ok(Template::render("rigging/equipment", context))
}
