use rocket::serde::json::Json;
use rocket::State;
use rocket::form::Form;
use rocket::response::{Stream, Redirect};

use stokepile_shared::staging::{UploadDescriptor, TrimDetail};
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
    name: String,
    trim_start: u32,
    trim_end: u32,
    max_trim: u32,
}

impl UpdateForm {
    fn as_trim_detail(&self) -> TrimDetail {
        TrimDetail {
            start: self.trim_start,
            end: self.trim_end,
        }
    }
}

// this lives in /api but isn't really an api per se since it's meant to be hit wiht a form post
#[post("/api/media/<uuid>/update", data = "<update>")]
pub fn update_media(staging: State<'_, MountedStaging>, uuid: UuidParam, update: Form<UpdateForm>) -> Option<Redirect> {
    // TODO(richo) add Flash to show the user success
    file_by_uuid(&staging, *uuid)
        .map(|mut file| {
            warn!("Found a file, {:?}", &file);
            if update.trim_start != 0 ||
                update.trim_end != update.max_trim {
                    let _ = file.add_trim(update.as_trim_detail());
            }
            file.update_name(&update.name)
                .expect("failed to rename");
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
pub fn apply_trims(staging: State<'_, MountedStaging>) -> Result<(), Error> {
    for file in staging.staged_files()? {
        let _ = file.apply_trim();
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
