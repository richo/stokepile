use std::env;
use std::fmt;
use std::ops::Deref;
use failure::Error;

use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection, Pool, PooledConnection};
use diesel::Connection;

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, Rocket, State};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

lazy_static! {
    static ref DATABASE_URL: String =
        env::var("DATABASE_URL").expect("DATABASE_URL is not set.");
}

pub fn init_pool(test_transactions: bool) -> PgPool {
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

/// Get a connection to the database. This is only for use in a standalone context, not within the
/// web server.
pub fn db_connection() -> Result<PgConnection, Error> {
    let conn = PgConnection::establish(&DATABASE_URL)?;
    Ok(conn)
}

pub fn run_migrations() -> Result<(), Error> {
    let mut conn = PgConnection::establish(&DATABASE_URL)?;
    diesel_migrations::run_pending_migrations(&mut conn)?;
    Ok(())
}

#[derive(Debug)]
struct TestTransactionCustomizer;

impl CustomizeConnection<PgConnection, r2d2::Error> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), r2d2::Error> {
        conn.begin_test_transaction().map_err(r2d2::Error::QueryError)
    }
}
