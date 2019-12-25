use diesel::prelude::*;
use super::*;

use crate::web::schema::customers;
use crate::web::routes::rigging::NewCustomerForm;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct Customer {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub address: String,
    pub phone_number: String,
    pub email: String,
}


#[derive(Insertable, Debug)]
#[table_name = "customers"]
pub struct NewCustomer<'a> {
    pub user_id: i32,
    pub name: &'a str,
    pub address: &'a str,
    pub phone_number: &'a str,
    pub email: &'a str,
}

impl Customer {
    // TODO(richo) paginate
    pub fn all(conn: &PgConnection, user_id: i32) -> QueryResult<Vec<Customer>> {
        use crate::web::schema::customers::dsl::*;
        customers.load::<Customer>(&*conn)
    }
}

// Should htis actually just be a Customer::create ?
impl<'a> NewCustomer<'a> {
    // Do we want some request global user_id? Seems positive but I also don't really see how to
    // make it happen.
    pub fn from(customer: &'a NewCustomerForm, user_id: i32) -> NewCustomer<'_> {
        NewCustomer {
            user_id,
            name: &customer.name,
            address: &customer.address,
            phone_number: &customer.phone_number,
            email: &customer.email,
        }
    }

    pub fn create(&self, conn: &PgConnection) -> QueryResult<Customer> {
        use diesel::insert_into;

        insert_into(customers::table)
            .values(self)
            .get_result::<Customer>(conn)
    }
}
