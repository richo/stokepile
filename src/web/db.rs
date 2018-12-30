use std::env;
use std::fmt;
use std::ops::Deref;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Error, Pool, PooledConnection};
use diesel::Connection;

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, Rocket, State};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

pub fn init_pool(test_transactions: bool) -> PgPool {
    lazy_static! {
        static ref DATABASE_URL: String =
            env::var("DATABASE_URL").expect("DATABASE_URL is not set.");
    }

    let manager = ConnectionManager::<PgConnection>::new(DATABASE_URL.clone());
    let mut builder = Pool::builder();

    if test_transactions {
        builder = builder
            .max_size(1)
            .connection_customizer(Box::new(TestTransactionCustomizer))
    }

    builder
        .build(manager)
        .expect("Could not initialize database pool.")
}

pub struct DbConn(pub PooledConnection<ConnectionManager<PgConnection>>);

impl fmt::Debug for DbConn {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple("DbConn")
            .field(&"PooledConnection<ConnectionManager<...>>")
            .finish()
    }
}

impl DbConn {
    pub fn maybe_from_rocket(rocket: &Rocket) -> Option<DbConn> {
        let pool = rocket.state::<PgPool>()?;
        match pool.get() {
            Ok(conn) => Some(DbConn(conn)),
            _ => None,
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let pool = request.guard::<State<'_, PgPool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

impl Deref for DbConn {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
struct TestTransactionCustomizer;

impl CustomizeConnection<PgConnection, Error> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), Error> {
        conn.begin_test_transaction().map_err(Error::QueryError)
    }
}
