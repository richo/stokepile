#![allow(proc_macro_derive_resolution_fallback)]
use rand;

mod user;
pub use self::user::{NewUser, User};

mod session;
pub use self::session::{NewSession, Session};

mod integration;
pub use self::integration::{Integration, NewIntegration};

mod device;
pub use self::device::{Device, NewDevice};

mod key;
pub use self::key::{Key, NewKey};

mod confirmation_tokens;
pub use self::confirmation_tokens::{ConfirmationToken, NewConfirmationToken};

pub mod extra {
    pub use super::user::{StagingKind, StagingKindMapping};
}

fn generate_secret() -> String {
    let (x, y) = rand::random::<(u64, u64)>();
    format!("{:x}{:x}", x, y)
}
