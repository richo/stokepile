pub mod support {
    use rocket_contrib::templates::Template;
    use crate::web::context::Context;

    #[get("/support/power")]
    pub fn power() -> Template {
        let context = Context::other();
        Template::render("support/power", context)
    }
}
