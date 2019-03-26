use rocket_contrib::templates::Template;
use crate::web::context::Context;

#[get("/healthcheck")]
pub fn healthcheck() -> Template {
    Template::render("healthcheck", Context::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;
    use rocket::http::Status;

    client_for_routes!(healthcheck => client);

    #[test]
    fn test_healthcheck() {
        init_env();

        let client = client();
        let req = client
            .get("/healthcheck");

        let response = req.dispatch();

        assert_eq!(response.status(), Status::Ok);
    }
}
