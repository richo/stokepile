use rocket_contrib::json::Json;
use rocket::State;
use rocket::response::Stream;
use uuid::Uuid;

use stokepile_shared::staging::UploadDescriptor;
use crate::staging::{MountedStaging, StagingLocation};
use crate::web::RangeResponder;

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
pub fn stream_media(staging: State<'_, MountedStaging>, uuid: String) -> Option<RangeResponder<File>> {
    let uuid = match Uuid::parse_str(&uuid) {
        Ok(uuid) => uuid,
        Err(e) => {
            warn!("Error parsing uuid: {:?}", e);
            return None;
        }
    };
    staging.staged_files()
        .expect("Couldn't load staged_files")
        .iter()
        .filter(|file| file.descriptor.uuid == uuid)
        .next()
        .and_then(|file| File::open(&file.content_path).ok())
        .map(|fh| RangeResponder::new(fh))
}
