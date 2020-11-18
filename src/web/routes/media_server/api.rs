use rocket_contrib::json::Json;
use rocket::State;
use rocket::response::Stream;

use stokepile_shared::staging::{UploadDescriptor, MediaTransform};
use crate::staging::{MountedStaging, StagingLocation, StagedFileExt};
use crate::web::RangeResponder;
use crate::web::form_hacks::UuidParam;

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
pub struct TrimForm {
    start: u64,
    end: u64,
}

#[post("/api/media/<uuid>/trim", format = "json", data = "<trim>")]
pub fn add_trim(staging: State<'_, MountedStaging>, uuid: UuidParam, trim: Json<TrimForm>) -> Option<()> {
    // TODO(richo) this should actually replace any existing trim
    staging.staged_files()
        .expect("Couldn't load staged_files")
        .iter_mut()
        .filter(|file| file.descriptor.uuid == *uuid)
        .next()
        .and_then(|file| file.add_transform(MediaTransform::trim(trim.start, trim.end)).ok())
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
