use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::PathBuf;
use failure::Error;
use stokepile_shared::staging::{
    StagedFile,
    TrimDetail,
    UploadDescriptor,
    AsTransform,
    RemotePathDescriptor,
    Trimmer,
    MediaTransform,
};
use dropbox_content_hasher::DropboxContentHasher;
use hashing_copy;
use crate::failure::ResultExt;

use std::process::{Command, Stdio};
use crate::staging::StagedFileExt;

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

static FFMPEG_CMD: &'static str = "ffmpeg";

impl FFMpegTrimmer {
    /// Create an ffmpeg trimmer. If the Err case is returned ffmpeg is either broken or
    /// nonexistant.
    // TODO(richo) Don't leak the io::Error
    pub fn new() -> Result<Self, io::Error> {
        info!("probing ffmpeg");
        Command::new(FFMPEG_CMD)
            .arg("-version")
            .output()
            .map(|_output| {
                info!("ffmpeg probe output: {:?}", _output);
                FFMpegTrimmer { _unused: () }
            })
    }
}

impl Trimmer for FFMpegTrimmer {
    fn trim(&self, file: StagedFile, detail: &TrimDetail) -> Result<StagedFile, (StagedFile, Error)> {
        let content_path = file.content_path.with_modification(detail);
        let mut manifest_path = content_path.clone();

        let mut file_name = manifest_path.file_name().expect("filename")
            .to_str().expect("to_str()").to_string();
        file_name.push_str(".manifest");
        manifest_path.set_file_name(file_name);

        let cleanup = || {
            let _ = fs::remove_file(&content_path);
            let _ = fs::remove_file(&manifest_path);
        };

        let res = (|| {
            let mut new = File::create(&content_path)
                .context("Creating new file")?;

            let mut content_hash = [0; 32];
            info!("Starting ffmpeg");
            let mut ffmpeg = Command::new(FFMPEG_CMD)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .arg("-i").arg(&file.content_path)
                .arg("-ss").arg(&detail.start_as_ffmpeg())
                .arg("-to").arg(&detail.end_as_ffmpeg())
                .arg("-c:v").arg("copy")
                .arg("-c:a").arg("copy")
                .arg("-f").arg("ismv") // TODO(richo)
                .arg("-")
                .spawn()
                .context("Command { ffmpeg }.spawn()")?;

            info!("Copying and hashing");
            let (size, hash) = hashing_copy::copy_and_hash::<_, _, DropboxContentHasher>(
                &mut ffmpeg.stdout.take().expect("stdout"),
                &mut new)?;
            content_hash.copy_from_slice(&hash);

            info!("waiting on ffmpeg");
            let res = ffmpeg.wait()?;
            if !res.success() {
                use std::io::Read;
                let mut error_text = String::new();
                let _ = ffmpeg.stderr.take().unwrap().read_to_string(&mut error_text);
                bail!("ffmpeg failed, output: {:?}", error_text);
            }

            let transforms = file.transforms.iter()
                .filter(|t| ! matches!(t, MediaTransform::Trim { .. }))
                .map(|t| t.clone())
                .collect();
            let descriptor = UploadDescriptor {
                path: file.descriptor.path.with_modification(detail),
                device_name: file.descriptor.device_name.clone(),
                content_hash,
                size,
                uuid: file.descriptor.uuid,
                transforms,
            };

            // We've now created the trimmed file, now just to make a manifest for it.

            let mut options = OpenOptions::new();
            let options = options.write(true).create(true).truncate(true);

            {
                let mut staged = options.open(&manifest_path)?;
                serde_json::to_writer(&mut staged, &descriptor)?;
            }

            Ok(StagedFile {
                content_path: content_path.clone(),
                manifest_path: manifest_path.clone(),
                descriptor,
            })
        })();

        match res {
            Ok(new_file) => {
                info!("Applying transform was successsful, erasing file");
                let _ = file.delete();
                Ok(new_file)
            }
            Err(err) => {
                info!("In error handler, clenaing up");
                cleanup();
                Err((file, err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use crate::staging::StagingLocation;
    use stokepile_shared::staging::MediaTransform;
    use uuid::Uuid;

    #[test]
    fn test_can_calc_name() {
        let pb: PathBuf = "/path/to/file.ext".into();
        let trim = pb.with_modification(&MediaTransform::trim(3, 6));
        let trimmed: PathBuf = "/path/to/file-trim-3:6.ext".into();
        assert_eq!(trimmed, trim);
    }

    #[test]
    fn test_path_calculation() {
        // stage a file
        let data = staged_data(1).expect("staged_data");
        let file = &data.staged_files().expect("staged_files")[0];
        let detail = MediaTransform::trim(1, 2);
        let new_path = file.content_path.with_modification(&detail);

        let mut content_hash = [0; 32];
        let descriptor = UploadDescriptor {
            path: file.descriptor.path.with_modification(&detail),
            device_name: "".into(),
            content_hash,
            size: 0,
            uuid: Uuid::new_v4(),
            transforms: vec![],
        };
    }
}
