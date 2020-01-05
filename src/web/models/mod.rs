#![allow(proc_macro_derive_resolution_fallback)]
use rand;

mod customer;
pub use self::customer::{NewCustomer, Customer};

mod component;
pub use self::component::{NewComponent, Component};

mod equipment;
pub use self::equipment::{NewEquipment, Equipment};

mod invite;
pub use self::invite::{NewInvite, Invite};

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

mod global_settings;
pub use self::global_settings::GlobalSetting;

pub mod extra {
    pub use super::user::{StagingKind, StagingKindMapping};
}

fn generate_secret() -> String {
    let (x, y) = rand::random::<(u64, u64)>();
    format!("{:x}{:x}", x, y)
}
