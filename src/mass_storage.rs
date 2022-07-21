use std::fs::{self, File};
use std::path::{Path, PathBuf};

use crate::config::{MassStorageConfig, MountableDeviceLocation};
use crate::mountable::{MountableFilesystem, MountedFilesystem, MountableKind};
use crate::staging::{StageFromDevice, DateTimeUploadable};

use chrono;
use chrono::prelude::*;
use failure::{Error, ResultExt};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct MountedMassStorage {
    mass_storage: MassStorageConfig,
    mount: MountedFilesystem,
}

#[derive(Debug)]
pub struct MassStorageFile {
    capturedatetime: DateTime<Local>,
    file: File,
    extension: String,
    source_path: PathBuf,
}

impl DateTimeUploadable for MassStorageFile {
    type Reader = File;

    fn extension(&self) -> &str {
        &self.extension
    }

    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError> {
        Ok(self.capturedatetime)
    }

    fn reader(&mut self) -> &mut File {
        &mut self.file
    }

    fn delete(&mut self) -> Result<(), Error> {
        fs::remove_file(&self.source_path)?;
        Ok(())
    }

    fn size(&self) -> Result<u64, Error> {
        Ok(self.file.metadata()?.len())
    }
}

impl StageFromDevice for MountedMassStorage {
    type FileType = MassStorageFile;

    fn files(&self) -> Result<Vec<MassStorageFile>, Error> {
        // Screw it
        let mut out = vec![];

        for ref path in self.files_matching_extensions() {
            // Could definitely lift this into some domain object
            let extension = path.extension().unwrap().to_str().unwrap().to_lowercase();
            let file = MassStorageFile {
                capturedatetime: path.metadata()?.modified()?.into(),
                file: File::open(path)
                    .context("Opening content file for MountedMassStorage")?,
                source_path: path.to_path_buf(),
                extension,
            };
            out.push(file);
        }
        Ok(out)
    }

    fn cleanup(&self) -> Result<(), Error> {
        for path in self.files_matching_cleanup_extensions() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

impl MountedMassStorage {
    /// Returns `PathBuf`s for all the files matching the `extensions` field on this MassStorage.
    pub fn files_matching_extensions(&self) -> Vec<PathBuf> {
        self.map_files_with_extensions(&self.mass_storage.extensions[..]).collect()
    }

    /// Returns `PathBuf`s for all the files matching the `cleanup_extensions` field on this MassStorage.
    pub fn files_matching_cleanup_extensions(&self) -> Vec<PathBuf> {
        match &self.mass_storage.cleanup_extensions {
            Some(extns) => {
                self.map_files_with_extensions(extns).collect()
            },
            None => {
                vec![]
            },
        }
    }


    fn map_files_with_extensions<'a>(&self, extensions: &'a [String]) -> impl Iterator<Item=PathBuf> + 'a {
        WalkDir::new(&self.mount.path())
            .into_iter()
            .filter_map(move |entry| {
                // .Trashes does some weird OSX thing, nfi why. We'll just have a peek
                // To answer the TODO below this block, yes.
                if let Err(ref e) = entry {
                    if e.path().map(|x| x.ends_with(".Trashes")) == Some(true) {
                        return None
                    }
                }

                // TODO(richo) Do we think this actually can fail?
                let entry = entry.unwrap();
                if entry.file_type().is_dir() {
                    return None;
                }

                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let extension = ext.to_str().unwrap().to_lowercase();
                    if !extensions.contains(&extension) {
                        return None;
                    }

                    if let Some(Some(filename)) = path.file_name().map(|s|s.to_str()) {
                        if filename.starts_with("._") {
                            return None
                        }
                    }

                    for anc in path.ancestors() {
                        if let Some(Some(dirname)) = anc.file_name().map(|s|s.to_str()) {
                            if dirname == ".Trashes" {
                                return None
                            }
                        }
                    }

                    return Some(path.to_path_buf());
                } else {
                    return None;
                }
            })
    }
}

impl MountableFilesystem for MassStorageConfig {
    type Target = MountedMassStorage;

    fn location(&self) -> &MountableDeviceLocation {
        &self.location
    }
}

impl MountableKind for MountedMassStorage {
    type This = MassStorageConfig;

    fn from_mounted_parts(this: Self::This, mount: MountedFilesystem) -> Self {
        MountedMassStorage {
            mass_storage: this,
            mount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::{self, FileTime};
    use crate::test_helpers;
    use walkdir;

    use std::path::Path;

    fn extensions() -> Vec<String> {
        vec!["mp4".into()]
    }

    /// Git checkouts will have mtimes super close together, which will break our algorithm.
    ///
    /// We probably want at some point to remove this test (And introduce the opposite- proving
    /// that we're durable to this) but for now we'll just skew them a bit.
    fn fix_filetimes(root: &Path) -> Result<(), Error> {
        for (i, entry) in walkdir::WalkDir::new(root).into_iter().enumerate() {
            let entry = entry.unwrap();
            if !entry.file_type().is_file() {
                continue;
            }
            let metadata = fs::metadata(entry.path())?;
            let mtime = FileTime::from_last_modification_time(&metadata);
            let unix_seconds = mtime.unix_seconds();

            let new = FileTime::from_unix_time(unix_seconds + (i as i64 * 10), 0);
            filetime::set_file_times(entry.path(), new, new)?;
        }
        Ok(())
    }

    #[test]
    fn test_mass_storage_loads_files() {
        let mass_storage = MassStorageConfig {
            name: "data".into(),
            location: MountableDeviceLocation::from_mountpoint("test-data/mass_storage".into()),
            extensions: extensions(),
            cleanup_extensions: None,
        };
        let mounted = mass_storage.mount_for_test();

        let files = mounted.files().expect("Couldn't load test files");
        assert_eq!(files.len(), 2);
        for file in files {
            assert_eq!(&file.extension, "mp4");
        }
    }

    #[test]
    fn test_staging_works() {
        let dest = test_helpers::temp_stager();
        let source = test_helpers::test_data("mass_storage");
        fix_filetimes(&source.path()).unwrap();

        let mass_storage = MassStorageConfig {
            name: "data".into(),
            location: MountableDeviceLocation::from_mountpoint(source.path().to_path_buf()),
            extensions: extensions(),
            cleanup_extensions: Some(vec!["lrv".into()]),
        };

        let mounted = mass_storage.mount_for_test();

        // Confirm that we see the lrv files before hand
        assert_eq!(mounted.files_matching_cleanup_extensions().len(), 2);

        let mounted = mounted.stage_files_for_test("data", &dest).unwrap();
        // TODO(richo) test harder
        let iter = fs::read_dir(&dest.staging_location()).unwrap();
        let files: Vec<_> = iter.collect();

        // Two files for the two mp4 files, two files for the manifests
        assert_eq!(files.len(), 4);

        // Assert that the original lrv's are gone.
        assert_eq!(mounted.files_matching_cleanup_extensions().len(), 0);
    }
}
