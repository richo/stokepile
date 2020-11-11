use rocket_contrib::json::Json;
use rocket::State;

use stokepile_shared::staging::UploadDescriptor;
use crate::staging::{MountedStaging, StagingLocation};

#[get("/api/media")]
pub fn get_media(staging: State<'_, MountedStaging>) -> Json<Vec<UploadDescriptor>> {
    let files = staging.staged_files()
        .expect("Couldn't load staged_files")
        .into_iter()
        .map(|x| x.1)
        .collect();
    Json(files)
}
