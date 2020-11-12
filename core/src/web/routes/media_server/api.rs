use hex;
use rocket_contrib::json::Json;
use rocket::State;
use rocket::response::Stream;

use stokepile_shared::staging::UploadDescriptor;
use crate::staging::{MountedStaging, StagingLocation};

use std::fs::File;

#[get("/api/media")]
pub fn get_media(staging: State<'_, MountedStaging>) -> Json<Vec<UploadDescriptor>> {
    let files = staging.staged_files()
        .expect("Couldn't load staged_files")
        .into_iter()
        .map(|x| x.1)
        .collect();
    Json(files)
}

#[get("/api/media/<hash>")]
pub fn stream_media(staging: State<'_, MountedStaging>, hash: String) -> Option<Stream<File>> {
    staging.staged_files()
        .expect("Couldn't load staged_files")
        .iter()
        .filter(|(_, d)| hex::encode(d.content_hash) == &hash[..])
        .next()
        .map(|(f, _)| File::open(&f.content_path).ok())
        .flatten()
        .map(|f| Stream::chunked(f, 4096))
}
