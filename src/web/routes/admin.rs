use rocket::{get, post};
use rocket_contrib::templates::Template;

use crate::web::auth::AdminUser;
use crate::web::context::Context;

#[get("/admin")]
pub fn index(user: AdminUser) -> Template {
    let context = Context::default()
        .set_user(Some(user.into()));
    Template::render("admin", context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;

    use rocket::http::Status;

    client_for_routes!(index => client);

    #[test]
    fn admin_loads_for_admins() {
        init_env();
        let client = client();
        let _admin = create_admin(&client, "test1@email.com", "p@55w0rd");
        let _session = signin(&client, "test1%40email.com", "p%4055w0rd").unwrap();

        let response = client
            .get("/admin")
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn admin_doesnt_load_for_non_admins() {
        init_env();
        let client = client();
        let _user = create_user(&client, "test1@email.com", "p@55w0rd");
        let _session = signin(&client, "test1%40email.com", "p%4055w0rd").unwrap();

        let response = client
            .get("/admin")
            .dispatch();
        assert_eq!(response.status(), Status::Unauthorized);
    }
}
