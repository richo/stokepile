use rocket_contrib::json::Json;
use rocket::State;
use rocket::request::{Form};
use rocket::response::{Stream, Redirect};

use stokepile_shared::staging::{UploadDescriptor, MediaTransform};
use crate::staging::{MountedStaging, StagingLocation, StagedFileExt, StagedFile};
use crate::web::RangeResponder;
use crate::web::form_hacks::UuidParam;

use uuid::Uuid;

use failure::Error;

use std::fs::File;

#[get("/api/media")]
pub fn get_media(staging: State<'_, MountedStaging>) -> Json<Vec<UploadDescriptor>> {
    let files = staging.staged_files()
        .expect("Couldn't load staged_files")
        .into_iter()
        .map(|x| x.descriptor)
        .collect();
    Json(files)
}

#[get("/api/media/<uuid>")]
pub fn stream_media(staging: State<'_, MountedStaging>, uuid: UuidParam) -> Option<RangeResponder<File>> {
    staging.staged_files()
        .expect("Couldn't load staged_files")
        .iter()
        .filter(|file| file.descriptor.uuid == *uuid)
        .next()
        .and_then(|file| File::open(&file.content_path).ok())
        .map(|fh| RangeResponder::new(fh))
}

#[derive(Debug, FromForm, Deserialize)]
pub struct UpdateForm {
    new_name: String,
    trim_start: u64,
    trim_end: u64,
}

// this lives in /api but isn't really an api per se since it's meant to be hit wiht a form post
#[post("/api/media/<uuid>/rename", data = "<rename>")]
pub fn update_media(staging: State<'_, MountedStaging>, uuid: UuidParam, rename: Form<UpdateForm>) -> Option<Redirect> {
    // TODO(richo) add Flash to show the user success
    file_by_uuid(&staging, *uuid)
        .map(|mut file| {
            if rename.trim_start != 0 ||
                rename.trim_end != 100 {
                    let _ = file.add_transform(MediaTransform::trim(rename.trim_start, rename.trim_end));
            }
            file.rename(rename.into_inner().new_name);
        })?;

    Some(Redirect::to("/"))
}

fn file_by_uuid(staging: &MountedStaging, uuid: Uuid) -> Option<StagedFile> {
    staging.staged_files()
        .expect("Couldn't load staged_files")
        .into_iter()
        .filter(|file| file.descriptor.uuid == uuid)
        .next()
}

#[post("/api/media/apply_transforms")]
pub fn apply_transforms(staging: State<'_, MountedStaging>) -> Result<(), Error> {
    for file in staging.staged_files()? {
        let _ = file.apply_transforms();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use crate::staging::{StageFromDevice, Stager};
    use crate::web::test_helpers::*;

    use rocket::http::{ContentType, Status};

    client_for_routes!(media: get_media, stream_media, add_trim => client);

    #[test]
    fn test_fetch_media() {
        let client = client();
        let device = DummyDataDevice::new(5);
        let staging = client.rocket().state::<MountedStaging>()
            .expect("staging");

        let stager = Stager::destructive(staging);

        device.stage_files("dummy", &stager)
            .expect("stage_files");

        let mut response = client
            .get("/api/media")
            .header(ContentType::JSON)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let body = &response.body_string().expect("didn't get a body");
        let media: Vec<UploadDescriptor> =
            serde_json::from_str(&body).expect("Couldn't deserialize");
        assert_eq!(media.len(), 5);
    }

    #[test]
    fn test_add_trim() {
        let client = client();
        let device = DummyDataDevice::new(1);
        let staging = client.rocket().state::<MountedStaging>()
            .expect("staging");

        let stager = Stager::destructive(staging);
        device.stage_files("dummy", &stager)
            .expect("stage_files");

        let file = &staging.staged_files().unwrap()[0];
        assert_eq!(file.transforms.len(), 0);

        let mut response = client
            .post(format!("/api/media/{}/trim", &file.descriptor.uuid))
            .header(ContentType::JSON)
            .body(format!("{{\"start\": 6, \"end\": 12}}"))
            .dispatch();

        let file = &staging.staged_files().unwrap()[0];
        assert_eq!(file.transforms[0],
            MediaTransform::trim(6, 12));
    }
}
