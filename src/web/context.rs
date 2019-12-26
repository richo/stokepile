use crate::web::auth::{AdminUser, WebUser};
use crate::web::models::User;

use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct PossibleIntegration {
    pub id: Option<i32>,
    pub name: &'static str,
    pub display_name: &'static str,
    pub connected: bool,
}

#[derive(Debug, Serialize)]
pub enum Module {
    Media,
    Rigging,
    Other,
}

#[derive(Serialize, Debug)]
/// The god object for handing into templates.
///
/// Individual templates can read what they need out of `data`, however type safety is mostly
/// discarded here and tests are required :(
pub struct Context<T: Serialize> {
    pub user: Option<WebUser>,
    pub signin_error: Option<String>,
    pub data: T,
    pub flash_message: Option<(String, String)>,
    pub module: Module,
}

// TODO(richo) migrate this back to using a Context and hanging off data
#[derive(Serialize, Debug)]
pub struct AdminContext<T: Serialize> {
    pub user: AdminUser,
    pub user_list: Option<Vec<User>>,
    pub flash_message: Option<(String, String)>,
    pub data: T,
    pub module: Module,
}

#[derive(Serialize, Debug)]
pub struct EmptyData {}

impl<T> Context<T>
where T: Serialize {
    pub fn rigging(data: T) -> Context<T> {
        Context {
            user: None,
            signin_error: None,
            data: data,
            flash_message: None,
            module: Module::Rigging,
        }
    }

    pub fn media(data: T) -> Context<T> {
        Context {
            user: None,
            signin_error: None,
            data: data,
            flash_message: None,
            module: Module::Media,
        }
    }

    pub fn set_signin_error(mut self, signin_error: Option<String>) -> Self {
        self.signin_error = signin_error;
        self
    }

    pub fn set_user(mut self, user: Option<WebUser>) -> Self {
        self.user = user;
        self
    }

    pub fn flash(
        mut self,
        flash_message: Option<(String, String)>,
    ) -> Self {
        self.flash_message = flash_message;
        self
    }
}

impl Context<EmptyData> {
    pub fn other() -> Context<EmptyData> {
        Context {
            user: None,
            signin_error: None,
            data: EmptyData{},
            flash_message: None,
            module: Module::Other,
        }
    }
}


impl<T> AdminContext<T>
where T: Serialize {
    pub fn for_user(user: AdminUser, data: T) -> Self {
        Self {
            user,
            user_list: None,
            flash_message: None,
            data,
            // TODO(richo) this doesn't necessarily follow
            module: Module::Media,
        }
    }

    pub fn set_user_list(mut self, users: Vec<User>) -> Self {
        self.user_list = Some(users);
        self
    }

    pub fn flash(
        mut self,
        flash_message: Option<(String, String)>,
    ) -> Self {
        self.flash_message = flash_message;
        self
    }
}
