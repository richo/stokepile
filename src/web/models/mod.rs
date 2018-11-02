#![allow(proc_macro_derive_resolution_fallback)]
use rand;

mod user;
pub use self::user::{User, NewUser};

mod session;
pub use self::session::{Session, NewSession};

mod integration;
pub use self::integration::{Integration, NewIntegration};

mod device;
pub use self::device::{Device, NewDevice};

mod key;
pub use self::key::{Key, NewKey};

fn generate_secret() -> String {
    let (x, y) = rand::random::<(u64, u64)>();
    format!("{:x}{:x}", x, y)
}
