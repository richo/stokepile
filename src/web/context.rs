use web::auth::CurrentUser;
use web::models::Device;

#[derive(Serialize, Debug)]
pub struct PossibleIntegration {
    pub id: Option<i32>,
    pub name: &'static str,
    pub display_name: &'static str,
    pub connected: bool,
}

#[derive(Serialize, Default, Debug)]
pub struct Context {
    pub user: Option<CurrentUser>,
    pub signin_error: Option<String>,
    pub integrations: Vec<PossibleIntegration>,
    pub devices: Vec<Device>,
    pub integration_message: Option<(String, String)>,
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

    pub fn set_integrations(mut self, integrations: Vec<PossibleIntegration>) -> Self {
        self.integrations = integrations;
        self
    }

    pub fn set_devices(mut self, devices: Vec<Device>) -> Self {
        self.devices = devices;
        self
    }

    pub fn set_integration_message(
        mut self,
        integration_message: Option<(String, String)>,
    ) -> Self {
        self.integration_message = integration_message;
        self
    }
}
