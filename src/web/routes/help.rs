use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::Context;

use rocket_dyn_templates::Template;

#[get("/help")]
pub fn help(user: WebUser, _conn: DbConn) -> Template {
    let context = Context::default()
        .set_user(Some(user));

    Template::render("help", context)
}

#[get("/beta")]
pub fn beta(user: WebUser, _conn: DbConn) -> Template {
    let context = Context::default()
        .set_user(Some(user));

    Template::render("beta", context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;

    use rocket::http::Status;

    client_for_routes!(config: beta, help => client);

    #[test]
    fn test_help_loads() {
        init_env();
        let client = client();
        let _user = create_user(&client, "test1@email.com", "p@55w0rd");
        let _session = signin(&client, "test1%40email.com", "p%4055w0rd").unwrap();

        let response = client
            .get("/help")
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn test_beta_loads() {
        init_env();
        let client = client();
        let _user = create_user(&client, "test1@email.com", "p@55w0rd");
        let _session = signin(&client, "test1%40email.com", "p%4055w0rd").unwrap();

        let response = client
            .get("/beta")
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
}
