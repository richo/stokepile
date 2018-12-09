use std::env;
use std::fmt;
use std::ops::Deref;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Error, Pool, PooledConnection};
use diesel::Connection;

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, Rocket, State};

#[database("maindb")]
pub struct DbConn(PgConnection);
