use web::auth::CurrentUser;

#[derive(Serialize, Default, Debug)]
pub struct Context {
    pub user: Option<CurrentUser>,
    pub signin_error: Option<String>,
}

impl Context {
    pub fn set_signin_error(mut self, signin_error: Option<String>) -> Self {
        self.signin_error = signin_error;
        self
    }

    pub fn set_user(mut self, user: Option<CurrentUser>) -> Self {
        self.user = user;
        self
    }
}
