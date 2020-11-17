use std::fs::File;
use std::io;
use std::path::PathBuf;
use failure::Error;
use stokepile_shared::staging::{
    StagedFile,
    TrimDetail,
    UploadDescriptor,
    AsTransform,
    RemotePathDescriptor,
};
use dropbox_content_hasher::DropboxContentHasher;
use hashing_copy;
use uuid::Uuid;

use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct FFMpegTrimmer {
    _unused: (),
}

// TODO(richo) should this be in the shared code?
trait PathWithTransform {
    type Output;
    fn with_modification<T: AsTransform>(&self, detail: &T) -> Self::Output;
}

impl PathWithTransform for PathBuf {
    type Output = PathBuf;
    fn with_modification<T: AsTransform>(&self, detail: &T) -> PathBuf {
        let tweak = detail.as_transform().tweak_name();

        let mut stem = self.file_stem().expect("file_stem")
            .to_str().expect("as_str").to_string();
        let extension = self.extension().expect("extension")
            .to_str().expect("as_str");
        let parent = self.parent().expect("parent");

        stem.push_str(&tweak);
        stem.push('.');
        stem.push_str(extension);

        parent.join(stem)
    }
}

impl PathWithTransform for RemotePathDescriptor {
    type Output = RemotePathDescriptor;
    fn with_modification<T: AsTransform>(&self, detail: &T) -> RemotePathDescriptor {
        use RemotePathDescriptor::*;
        let tweak = detail.as_transform().tweak_name();

        match self {
            // It will be a huge pain to support the {date}/{time}-trim.extension form, so for now
            // we're just going to ignore it and move on with our lives, even though that's also
            // the form that I most often use. The real risk here is that since they're unmodified
            // if we don't clean up properly there's a race with the trim and the original.
            dt @ DateTime { .. } => dt.clone(),
            DateName { capture_date, name, extension } => {
                let mut new_name = name.clone();
                new_name.push_str(&tweak);
                DateName {
                    capture_date: capture_date.clone(),
                    name: new_name,
                    extension: extension.clone(),
                }
            },
            SpecifiedPath { group, name, extension } => {
                let mut new_name = name.clone();
                new_name.push_str(&tweak);
                SpecifiedPath {
                    group: group.clone(),
                    name: new_name,
                    extension: extension.clone(),
                }
            }
        }
    }
}

impl FFMpegTrimmer {
    /// Create an ffmpeg trimmer. If the Err case is returned ffmpeg is either broken or
    /// nonexistant.
    // TODO(richo) Don't leak the io::Error
    pub fn new() -> Result<Self, io::Error> {
        Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|_output| FFMpegTrimmer { _unused: () })
    }

    pub fn trim(file: StagedFile, detail: TrimDetail) -> Result<StagedFile, Error> {
        let old = File::open(&file.content_path)?;
        let mut new = File::create(file.content_path.with_modification(&detail))?;
        let mut content_hash = [0; 32];
        let ffmpeg = Command::new("ffmpeg")
            .stdin(old)
            .stdout(Stdio::piped())
            .env_clear()
            .spawn()?;

        let (size, hash) = hashing_copy::copy_and_hash::<_, _, DropboxContentHasher>(
            &mut ffmpeg.stdout.expect("stdout"),
            &mut new)?;
        content_hash.copy_from_slice(&hash);

        let descriptor = UploadDescriptor {
            path: file.descriptor.path.with_modification(&detail),
            device_name: file.descriptor.device_name,
            content_hash,
            size,
            uuid: Uuid::new_v4(),
        };

        // We've now created the trimmed file, now just to make a manifest for it.




        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_calc_name() {
        let pb: PathBuf = "/path/to/file.ext".into();
        let trim = pb.with_modification(&MediaTransform::trim(3, 6));
        let trimmed: PathBuf = "/path/to/file-trim-3:6.ext".into();
        assert_eq!(trimmed, trim);
    }
}
