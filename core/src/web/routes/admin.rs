use rocket::{get, post};
use rocket::request::{FlashMessage, Form};
use rocket::response::{Flash, Redirect};
use rocket_contrib::templates::Template;

use crate::web::db::DbConn;
use crate::web::auth::AdminUser;
use crate::web::context::AdminContext;
use crate::web::models::{NewInvite, User};

#[get("/admin")]
pub fn index(user: AdminUser, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let context = AdminContext::for_user(user)
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("admin", context)
}

#[derive(FromForm, Debug, Serialize)]
pub struct InviteForm {
    email: String,
}

#[post("/admin/invite", data = "<invite>")]
pub fn create_invite(user: AdminUser, conn: DbConn, invite: Form<InviteForm>) -> Flash<Redirect> {
    match NewInvite::new(&invite.email).create(&*conn) {
        Ok(_) => {
            Flash::success(
                Redirect::to("/admin"),
                format!("Successfully created invite for {:?}", &invite.email),
                )
        },
        Err(e) => {
            Flash::error(
                Redirect::to("/admin"),
                format!("Error creating invite, {:?}", e),
                )
        }
    }
}

#[get("/admin/users")]
pub fn users(user: AdminUser, conn: DbConn, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let users = User::all(&conn).expect("loool");
    let context = AdminContext::for_user(user)
        // TODO(richo) error handling.
        .set_user_list(users)
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("admin/users", context)
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
