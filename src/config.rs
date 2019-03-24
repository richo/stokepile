use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use failure::Error;
use toml;
use url;

use crate::dropbox;
use crate::flysight::Flysight;
use crate::local_backup::LocalBackup;
use crate::mailer::SendgridMailer;
use crate::mass_storage::MassStorage;
use crate::pushover_notifier::PushoverNotifier;
use crate::staging::{StagingDirectory, StageableLocation};
use crate::storage::StorageAdaptor;
use crate::vimeo::VimeoClient;


// TODO(richo) Change this once we have a canonical domain
pub static DEFAULT_API_BASE: &'static str = "https://onatopp.psych0tik.net";
pub static TOKEN_FILE_NAME: &'static str = ".archiver-token";

#[derive(Debug)]
pub struct AccessToken(String);

#[cfg(test)]
lazy_static! {
    static ref HOME_DIR: tempfile::TempDir = tempfile::tempdir().unwrap();
}

/// Find the users home directory.
///
/// In tests, we persiste a tempdir over the process to ensure we're not spamming the user.
pub fn get_home() -> Result<impl AsRef<Path>, Error> {
    #[cfg(not(test))]
    let home_func = dirs::home_dir;
    #[cfg(test)]
    let home_func = || Some(&*HOME_DIR);

    match home_func() {
        Some(home) => Ok(home),
        None => {
            Err(format_err!(
                "Couldn't find your home directory. Is HOME set?"
            ))
        }
    }
}

impl AccessToken {
    pub fn save(token: &str) -> Result<(), Error> {
        AccessToken::save_with_dir_fn(get_home, token)
    }

    fn save_with_dir_fn<F, T>(home: F, token: &str) -> Result<(), Error>
    where
        F: Fn() -> Result<T, Error>,
        T: AsRef<Path>,
    {
        let mut file = File::create(home()?.as_ref().join(TOKEN_FILE_NAME))?;
        file.write_all(token.as_bytes())?;
        Ok(())
    }

    pub fn load() -> Result<Self, Error> {
        AccessToken::load_with_dir_fn(get_home)
    }

    fn load_with_dir_fn<F, T>(home: F) -> Result<Self, Error>
    where
        F: Fn() -> Result<T, Error>,
        T: AsRef<Path>,
    {
        let mut token = String::new();
        let mut file = File::open(home()?.as_ref().join(TOKEN_FILE_NAME))
            .map_err(|_| ConfigError::NoTokenFile)?;
        file.read_to_string(&mut token)?;
        Ok(AccessToken(token))
    }

    pub fn as_authorization_header(&self) -> String {
        format!("Bearer: {}", &self.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum DeviceConfig {
    Gopro(GoproConfig),
    MassStorage(MassStorageConfig),
    Flysight(FlysightConfig),
    UnknownDevice(String),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct Config {
    archiver: ArchiverConfig,
    dropbox: Option<DropboxConfig>,
    vimeo: Option<VimeoConfig>,
    // youtube: Option<YoutubeConfig>,
    flysight: Option<Vec<FlysightConfig>>,
    gopro: Option<Vec<GoproConfig>>,
    mass_storage: Option<Vec<MassStorageConfig>>,
    local_backup: Option<Vec<LocalBackupConfig>>,
    // gswoop: Option<GswoopConfig>,
    sendgrid: Option<SendgridConfig>,
    pushover: Option<PushoverConfig>,
}

#[derive(Debug, Default)]
pub struct ConfigBuilder {
    archiver: ArchiverConfig,
    dropbox: Option<DropboxConfig>,
    vimeo: Option<VimeoConfig>,
    // youtube: Option<YoutubeConfig>,
    flysight: Option<Vec<FlysightConfig>>,
    gopro: Option<Vec<GoproConfig>>,
    mass_storage: Option<Vec<MassStorageConfig>>,
    local_backup: Option<Vec<LocalBackupConfig>>,
    // gswoop: Option<GswoopConfig>,
    sendgrid: Option<SendgridConfig>,
    pushover: Option<PushoverConfig>,
}

lazy_static! {
    static ref EMPTY_MASS_STORAGES: Vec<MassStorageConfig> = vec![];
    static ref EMPTY_FLYSIGHTS: Vec<FlysightConfig> = vec![];
    static ref EMPTY_GOPROS: Vec<GoproConfig> = vec![];
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
/// The configuration entry associated with a staging location.
pub enum StagingConfig {
    #[serde(rename = "staging_directory")]
    StagingDirectory(PathBuf),
    #[serde(rename = "staging_device")]
    StagingDevice(PathBuf),
}

#[cfg(feature = "web")]
use crate::web::models::extra::StagingKind;

impl StagingConfig {
    #[cfg(feature = "web")]
    pub fn location(&self) -> &Path {
        match self {
            StagingConfig::StagingDirectory(buf) |
            StagingConfig::StagingDevice(buf) => &buf
        }
    }

    #[cfg(feature = "web")]
    pub fn kind(&self) -> StagingKind {
        match self {
            StagingConfig::StagingDirectory(_) => StagingKind::Directory,
            StagingConfig::StagingDevice(_) => StagingKind::Device,
        }
    }
}

impl StagingConfig {
    fn is_relative(&self) -> bool {
        match &*self {
            StagingConfig::StagingDirectory(path) |
            StagingConfig::StagingDevice(path) => path.is_relative()
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ArchiverConfig {
    #[serde(flatten)]
    staging: Option<StagingConfig>,
    api_base: Option<String>,
    api_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct DropboxConfig {
    token: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct VimeoConfig {
    token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub enum MountableDeviceLocation {
    #[serde(rename = "mountpoint")]
    Mountpoint(PathBuf),
    #[serde(rename = "label")]
    Label(String),
}

impl MountableDeviceLocation {
    pub fn from_mountpoint(pb: PathBuf) -> MountableDeviceLocation {
        MountableDeviceLocation::Mountpoint(pb)
    }

    pub fn from_label(lbl: String) -> MountableDeviceLocation {
        MountableDeviceLocation::Label(lbl)
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct FlysightConfig {
    pub name: String,
    #[serde(flatten)]
    pub location: MountableDeviceLocation,
}

impl FlysightConfig {
    pub fn flysight(&self) -> Flysight {
        Flysight::new(self.name.clone(),
                          self.location.clone())
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct LocalBackupConfig {
    pub mountpoint: String,
}

impl LocalBackupConfig {
    pub fn local_backup(&self) -> LocalBackup {
        // TODO(richo) barf if there's nothing mounted exactly here?
        LocalBackup {
            destination: PathBuf::from(self.mountpoint.clone()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct MassStorageConfig {
    pub name: String,
    pub mountpoint: String,
    pub extensions: Vec<String>,
}

impl MassStorageConfig {
    pub fn mass_storage(&self) -> MassStorage {
        MassStorage {
            name: self.name.clone(),
            path: PathBuf::from(self.mountpoint.clone()),
            extensions: self.extensions.iter().map(|x| x.to_lowercase()).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct PushoverConfig {
    pub token: String,
    pub recipient: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct SendgridConfig {
    pub token: String,
    pub from: String,
    pub to: String,
    pub subject: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct GoproConfig {
    pub name: String,
    pub serial: String,
}

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "Must have at least one of dropbox and vimeo configured.")]
    MissingBackend,
    #[fail(display = "Must have either a `staging_path` or `staging_device` set.")]
    MissingStaging,
    #[fail(display = "`staging_path` or `staging_device` must be absolute paths.")]
    RelativeStaging,
    #[fail(display = "Could not parse config: {}.", _0)]
    ParseError(#[cause] toml::de::Error),
    #[fail(display = "Could not generate config: {}.", _0)]
    GenerateError(#[cause] toml::ser::Error),
    #[fail(display = "Invalid url for api base: {}.", _0)]
    InvalidApiBase(url::ParseError),
    #[fail(display = "The token file does not exist. Did you login?")]
    NoTokenFile,
}

impl FromStr for Config {
    type Err = Error;

    fn from_str(body: &str) -> Result<Self, Self::Err> {
        match toml::from_str(body) {
            Ok(config) => Self::check_config(config),
            Err(e) => Err(ConfigError::ParseError(e))?,
        }
    }
}


impl Config {
    pub fn from_file(path: &str) -> Result<Config, Error> {
        let mut fh = File::open(path)?;
        let mut contents = String::new();
        fh.read_to_string(&mut contents)?;

        contents.parse()
    }

    /// Get a ConfigBuilder with which you can construct a Config object
    pub fn build() -> ConfigBuilder {
        Default::default()
    }

    /// Serializes this config option into TOML
    pub fn to_toml(&self) -> Result<String, Error> {
        toml::to_string(self).map_err(|e| ConfigError::GenerateError(e).into())
    }

    fn check_config(config: Config) -> Result<Config, Error> {
        if config.dropbox.is_none() && config.vimeo.is_none() {
            Err(ConfigError::MissingBackend)?;
        }

        if config.archiver.staging.is_none() {
            bail!(ConfigError::MissingStaging);
        }

        if let Some(staging) = &config.archiver.staging {
            if staging.is_relative() {
                Err(ConfigError::RelativeStaging)?
            }
        }

        if let Some(base) = &config.archiver.api_base {
            if let Err(err) = url::Url::parse(&base) {
                Err(ConfigError::InvalidApiBase(err))?;
            }
        }

        Ok(config)
    }

    /// Get the api base of this config, or return the default
    pub fn api_base(&self) -> &str {
        match &self.archiver.api_base {
            Some(base) => &base,
            None => DEFAULT_API_BASE,
        }
    }

    // Do we eventually want to make a camera/mass_storage distinction?
    pub fn mass_storages(&self) -> &Vec<MassStorageConfig> {
        match self.mass_storage {
            None => &EMPTY_MASS_STORAGES,
            Some(ref v) => v,
        }
    }

    pub fn flysights(&self) -> &Vec<FlysightConfig> {
        match self.flysight {
            None => &EMPTY_FLYSIGHTS,
            Some(ref v) => v,
        }
    }

    pub fn gopros(&self) -> &Vec<GoproConfig> {
        match self.gopro {
            None => &EMPTY_GOPROS,
            Some(ref v) => v,
        }
    }

    pub fn notifier(&self) -> Option<PushoverNotifier> {
        if let Some(ref pshvr) = self.pushover {
            return Some(PushoverNotifier::new(
                pshvr.token.clone(),
                pshvr.recipient.clone(),
            ));
        }
        None
    }

    pub fn mailer(&self) -> Option<SendgridMailer> {
        if let Some(ref sndgrd) = self.sendgrid {
            return Some(SendgridMailer::new(
                sndgrd.token.clone(),
                sndgrd.from.clone(),
                sndgrd.to.clone(),
                sndgrd.subject.clone(),
            ));
        }
        None
    }

    /// Returns a vec of all configured backends
    pub fn backends(&self) -> Vec<Box<dyn StorageAdaptor<File>>> {
        let mut out: Vec<Box<dyn StorageAdaptor<File>>> = vec![];
        if let Some(ref locals) = self.local_backup {
            for adaptor in locals {
                out.push(Box::new(adaptor.local_backup()));
            }
        }
        if let Some(ref dropbox) = self.dropbox {
            out.push(Box::new(dropbox::DropboxFilesClient::new(
                dropbox.token.clone(),
            )));
        }
        if let Some(ref vimeo) = self.vimeo {
            out.push(Box::new(VimeoClient::new(vimeo.token.clone())));
        }
        out
    }

    /// Returns an owned reference to the staging directory, expanded to be absolute
    pub fn staging(&self) -> Result<impl StageableLocation, Error> {
        match self.archiver.staging {
            Some(StagingConfig::StagingDirectory(ref path)) if path.is_absolute() => {
                Ok(StagingDirectory::new(path.to_path_buf()))
            },
            Some(StagingConfig::StagingDevice(ref device_path)) if device_path.is_absolute() => {
                unimplemented!()
            },
            Some(_) => {
                // It shouldn't be possible to get an absolute path because of the guards in
                // check_config, but it's super bad if we end up with one so we guard against it
                // very explicitly.
                bail!(ConfigError::RelativeStaging)
            }
            None => unreachable!("Should not be possible to construct a Config without staging set"),
        }
    }
}

impl ConfigBuilder {
    /// Set the staging directory for this config object
    pub fn staging(mut self, staging: StagingConfig) -> Self {
        self.archiver.staging = Some(staging);
        self
    }

    /// Set the dropbox API key for this object. This enables dropbox support.
    pub fn dropbox(mut self, token: String) -> Self {
        self.dropbox = Some(DropboxConfig { token });
        self
    }

    /// Set the vimeo API key for this object. This enables vimeo support.
    pub fn vimeo(mut self, token: String) -> Self {
        self.vimeo = Some(VimeoConfig { token });
        self
    }

    /// Add this flysight to the config object
    pub fn flysight(mut self, flysight: FlysightConfig) -> Self {
        let mut flysights = self.flysight.unwrap_or_else(|| vec![]);
        flysights.push(flysight);
        self.flysight = Some(flysights);
        self
    }

    /// Add multiple flysights to this config
    pub fn flysights(self, flysights: Vec<FlysightConfig>) -> Self {
        flysights
            .into_iter()
            .fold(self, |cfg, flysight| cfg.flysight(flysight))
    }

    /// Add this mass_storage to the config object
    pub fn mass_storage(mut self, mass_storage: MassStorageConfig) -> Self {
        let mut mass_storages = self.mass_storage.unwrap_or_else(|| vec![]);
        mass_storages.push(mass_storage);
        self.mass_storage = Some(mass_storages);
        self
    }

    /// Add multiple mass_storages to this config
    pub fn mass_storages(self, mass_storages: Vec<MassStorageConfig>) -> Self {
        mass_storages
            .into_iter()
            .fold(self, |cfg, mass_storage| cfg.mass_storage(mass_storage))
    }

    /// Add this gopro to the config object
    pub fn gopro(mut self, gopro: GoproConfig) -> Self {
        let mut gopros = self.gopro.unwrap_or_else(|| vec![]);
        gopros.push(gopro);
        self.gopro = Some(gopros);
        self
    }

    /// Add multiple gopros to this config
    pub fn gopros(self, gopros: Vec<GoproConfig>) -> Self {
        gopros.into_iter().fold(self, |cfg, gopro| cfg.gopro(gopro))
    }

    /// Add a local backup to this config
    pub fn local_backup(mut self, local_backup: LocalBackupConfig) -> Self {
        let mut local_backups = self.local_backup.unwrap_or(vec![]);
        local_backups.push(local_backup);
        self.local_backup = Some(local_backups);
        self
    }

    /// Add multiple gopros to this config
    pub fn local_backups(self, local_backups: Vec<LocalBackupConfig>) -> Self {
        local_backups.into_iter().fold(self, |cfg, local_backup| cfg.local_backup(local_backup))
    }

    /// Configure and enable pushover for this config
    pub fn pushover(mut self, pushover: PushoverConfig) -> Self {
        self.pushover = Some(pushover);
        self
    }

    /// Configure and enable sendgrid for this config
    pub fn sendgrid(mut self, sendgrid: SendgridConfig) -> Self {
        self.sendgrid = Some(sendgrid);
        self
    }

    /// Finalise this config object
    pub fn finish(self) -> Result<Config, Error> {
        Config::check_config(Config {
            archiver: self.archiver,
            dropbox: self.dropbox,
            vimeo: self.vimeo,
            flysight: self.flysight,
            gopro: self.gopro,
            local_backup: self.local_backup,
            mass_storage: self.mass_storage,
            sendgrid: self.sendgrid,
            pushover: self.pushover,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    #[test]
    fn test_example_config_parses() {
        let config = Config::from_file("archiver.toml.example").unwrap();

        assert_eq!(
            config.archiver,
            ArchiverConfig {
                api_token: Some("ARCHIVER_TOKEN_GOES_HERE".into()),
                api_base: Some("https://test-api.base".into()),
                staging: Some(StagingConfig::StagingDirectory("/test/staging/dir".into())),
            }
        );

        assert_eq!(
            config.dropbox,
            Some(DropboxConfig {
                token: "DROPBOX_TOKEN_GOES_HERE".into()
            })
        );

        assert_eq!(
            config.flysight,
            Some(vec![FlysightConfig {
                name: "data".into(),
                mountpoint: "/mnt/archiver/flysight".into(),
            }])
        );

        assert_eq!(
            config.mass_storage,
            Some(vec![MassStorageConfig {
                name: "video".into(),
                mountpoint: "/mnt/archiver/mass_storage".into(),
                extensions: vec!["mp4".into()],
            }])
        );

        assert_eq!(
            config.sendgrid,
            Some(SendgridConfig {
                token: "TOKEN_GOES_HERE".into(),
                from: "richo@example.net".into(),
                to: "richo@example.org".into(),
                subject: "archiver upload report".into(),
            })
        );

        assert_eq!(
            config.pushover,
            Some(PushoverConfig {
                token: "TOKEN_GOES_HERE".into(),
                recipient: "USER_TOKEN_GOES_HERE".into(),
            })
        );

        assert_eq!(
            config.vimeo,
            Some(VimeoConfig {
                token: "VIMEO_TOKEN_GOES_HERE".into(),
            })
        );
    }

    #[test]
    fn test_relative_staging() {
        let cfg = Config::from_str(
            r#"
[archiver]
staging_directory="test/dir"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap_err();
        let err = cfg.downcast::<ConfigError>().unwrap();
        let formatted = format!("{:?}", err);
        assert_eq!("RelativeStaging", &formatted);
    }

    #[test]
    fn test_staging_directory() {
        let cfg = Config::from_str(
            r#"
[archiver]
staging_directory="/test/dir"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap();
        assert_eq!(cfg.archiver.staging, Some(StagingConfig::StagingDirectory(PathBuf::from("/test/dir"))));
    }

    #[test]
    fn test_staging_device() {
        let cfg = Config::from_str(
            r#"
[archiver]
staging_device="/dev/staging"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap();
        assert_eq!(cfg.archiver.staging, Some(StagingConfig::StagingDevice(PathBuf::from("/dev/staging"))));
    }

    #[test]
    // This is a lie, it shouldn't fail. This is a bug.
    // TODO(richo)
    #[should_panic]
    fn test_staging_cannot_be_both() {
        let cfg = Config::from_str(
            r#"
[archiver]
staging_device="/dev/staging"
staging_directory="/test/dir"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap_err();
        let err = cfg.downcast::<ConfigError>().unwrap();
        assert!(match err {
            ConfigError::ParseError(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_invalid_api_base() {
        let cfg = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"
api_base = "malformed"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap_err();
        let err = cfg.downcast::<ConfigError>().unwrap();
        let formatted = format!("{:?}", err);
        assert_eq!("InvalidApiBase(RelativeUrlWithoutBase)", &formatted);
    }

    #[test]
    fn test_single_backend() {
        let cfg = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap();
        assert_eq!(cfg.backends().len(), 1);
    }

    #[test]
    fn test_multiple_backends() {
        let cfg = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"

[dropbox]
token = "TOKEN"

[vimeo]
token = "TOKEN"
"#,
        )
        .unwrap();
        assert_eq!(cfg.backends().len(), 2);
    }

    #[test]
    fn test_pushover() {
        let cfg = Config::from_str(
            r#"
[archiver]
staging_directory="/test/dir"

[dropbox]
token = "TOKEN"

[pushover]
token = "PUSHOVER_TOKEN"
recipient = "RECIPIENT_TOKEN"
"#,
        )
        .unwrap();
        assert!(cfg.notifier().is_some(), "Couldn't construct notifier");
    }

    #[test]
    fn test_no_backends() {
        let error = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"
"#,
        )
        .unwrap_err();
        println!("{:?}", &error);
        let error = error.downcast::<ConfigError>().unwrap_or_else(|e| panic!("{:?}", e));
        assert!(match error {
            ConfigError::MissingBackend => true,
            _ => false,
        });
    }

    #[test]
    fn test_no_peripherals() {
        let config = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"
"#,
        )
        .unwrap();
        assert_no_mass_storages(&config);
        assert_no_flysights(&config);
    }

    fn assert_no_mass_storages(cfg: &Config) {
        assert_eq!(cfg.mass_storage, None);
        assert_eq!(cfg.mass_storages(), &vec![]);
    }

    fn assert_no_flysights(cfg: &Config) {
        assert_eq!(cfg.flysight, None);
        assert_eq!(cfg.flysights(), &vec![]);
    }

    fn assert_mass_storages(cfg: &Config) {
        assert_eq!(
            cfg.mass_storages(),
            &vec![
                MassStorageConfig {
                    name: "front".into(),
                    mountpoint: "/mnt/archiver/front".into(),
                    extensions: vec!["mp4".into()],
                },
                MassStorageConfig {
                    name: "back".into(),
                    mountpoint: "/mnt/archiver/back".into(),
                    extensions: vec!["mov".into()],
                }
            ]
        )
    }

    fn assert_gopros(cfg: &Config) {
        assert_eq!(
            cfg.gopros(),
            &vec![
                GoproConfig {
                    name: "gopro4".into(),
                    serial: "C3131127500000".into(),
                },
                GoproConfig {
                    name: "gopro5".into(),
                    serial: "C3131127500001".into(),
                }
            ]
        )
    }

    fn assert_flysights(cfg: &Config) {
        assert_eq!(
            cfg.flysights(),
            &vec![
                FlysightConfig {
                    name: "training".into(),
                    mountpoint: "/mnt/archiver/training".into(),
                },
                FlysightConfig {
                    name: "comp".into(),
                    mountpoint: "/mnt/archiver/comp".into(),
                }
            ]
        )
    }

    #[test]
    fn test_mass_storages() {
        let config = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[mass_storage]]
name = "front"
mountpoint="/mnt/archiver/front"
extensions = ["mp4"]

[[mass_storage]]
name = "back"
mountpoint="/mnt/archiver/back"
extensions = ["mov"]
"#,
        )
        .unwrap();
        assert_mass_storages(&config);
        assert_no_flysights(&config);
    }

    #[test]
    fn test_gopros() {
        let config = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[gopro]]
name = "gopro4"
serial = "C3131127500000"

[[gopro]]
name = "gopro5"
serial = "C3131127500001"
"#,
        )
        .unwrap();
        assert_gopros(&config);
        assert_no_mass_storages(&config);
        assert_no_flysights(&config);
    }

    #[test]
    fn test_flysights() {
        let config = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[flysight]]
name = "training"
mountpoint="/mnt/archiver/training"

[[flysight]]
name = "comp"
mountpoint="/mnt/archiver/comp"
"#,
        )
        .unwrap();
        assert_flysights(&config);
        assert_no_mass_storages(&config);
    }

    #[test]
    fn test_mass_storages_and_flysights() {
        let config = Config::from_str(
            r#"
[archiver]
staging_directory = "/test"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[mass_storage]]
name = "front"
mountpoint="/mnt/archiver/front"
extensions = ["mp4"]

[[mass_storage]]
name = "back"
mountpoint="/mnt/archiver/back"
extensions = ["mov"]

[[flysight]]
name = "training"
mountpoint="/mnt/archiver/training"

[[flysight]]
name = "comp"
mountpoint="/mnt/archiver/comp"
"#,
        )
        .unwrap();
        assert_mass_storages(&config);
        assert_flysights(&config);
    }

    #[test]
    fn test_save_token() {
        let dir = test_helpers::tempdir();
        let token = "test-token";
        let mut saved_token = String::new();
        AccessToken::save_with_dir_fn(|| Ok(dir.path()), token).unwrap();
        File::open(dir.path().join(TOKEN_FILE_NAME))
            .unwrap()
            .read_to_string(&mut saved_token)
            .unwrap();
        assert_eq!(&saved_token, token);
    }

    #[test]
    fn test_load_token() {
        let dir = test_helpers::tempdir();
        let token = "test-token";
        File::create(dir.path().join(TOKEN_FILE_NAME))
            .unwrap()
            .write(token.as_bytes())
            .unwrap();
        assert_eq!(
            AccessToken::load_with_dir_fn(|| Ok(dir.path())).unwrap().0,
            token
        );
    }

    #[test]
    fn test_nice_error_for_nonexistant_token() {
        let dir = test_helpers::tempdir();
        let token_error = AccessToken::load_with_dir_fn(|| Ok(dir.path())).unwrap_err();
        let inner_error = token_error.downcast::<ConfigError>().unwrap();
        assert!(
            match inner_error {
                ConfigError::NoTokenFile => true,
                _ => false,
            },
            "Didn't get the correct error"
        );
    }
}
