use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::fmt;

use failure::Error;
use toml;
use url;

use crate::dropbox;
use crate::mailer::SendgridMailer;
use crate::pushover_notifier::{Notify, PushoverNotifier};
use crate::web_notifier::WebNotifier;
use crate::vimeo::VimeoClient;
use crate::mountable::{Mountable, MountableFilesystem};
use crate::storage::MaybeStorageAdaptor;


// TODO(richo) Change this once we have a canonical domain
pub static DEFAULT_API_BASE: &'static str = "https://archiver-web.onrender.com/";
pub static TOKEN_FILE_NAME: &'static str = ".archiver-token";

#[derive(RedactedDebug)]
pub struct AccessToken(#[redacted] String);

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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    archiver: ArchiverConfig,
    staging: StagingConfig,
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
    web_notifications: Option<WebNotificationsConfig>,
}

#[derive(Debug, Default)]
pub struct ConfigBuilder {
    archiver: ArchiverConfig,
    staging: Option<StagingConfig>,
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
    web_notifications: Option<WebNotificationsConfig>,
}

lazy_static! {
    static ref EMPTY_MASS_STORAGES: Vec<MassStorageConfig> = vec![];
    static ref EMPTY_FLYSIGHTS: Vec<FlysightConfig> = vec![];
    static ref EMPTY_GOPROS: Vec<GoproConfig> = vec![];
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
/// The configuration entry associated with a staging location.
pub struct StagingConfig {
    #[serde(flatten)]
    pub(crate) location: MountableDeviceLocation,
}

#[cfg(feature = "web")]
use crate::web::models::extra::StagingKind;

impl StagingConfig {
    #[cfg(feature = "web")]
    pub fn data_for_db(&self) -> String {
        match &self.location {
            MountableDeviceLocation::Label(buf) => buf.to_string(),
            MountableDeviceLocation::Mountpoint(buf) => buf.to_string_lossy().into(),
        }
    }

    #[cfg(feature = "web")]
    pub fn kind_for_db(&self) -> StagingKind {
        match &self.location {
            MountableDeviceLocation::Label(_) => StagingKind::Label,
            MountableDeviceLocation::Mountpoint(_) => StagingKind::Mountpoint,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArchiverConfig {
    api_base: Option<String>,
    api_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DropboxConfig {
    token: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VimeoConfig {
    token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub enum MountableDeviceLocation {
    // TODO(richo) This doens't really distinguish between a given mountpoint we should try to
    // mount, and a place to find things that will already be mounted, although I think the first
    // usecase is kinda deprecated anyway.
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

impl fmt::Display for MountableDeviceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MountableDeviceLocation::Mountpoint(path) => {
                write!(f, "Mountpoint({:?})", path)
            },
            MountableDeviceLocation::Label(label) => {
                write!(f, "Label({})", label)
            },
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct FlysightConfig {
    pub name: String,
    #[serde(flatten)]
    pub location: MountableDeviceLocation,
}

impl FlysightConfig {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct LocalBackupConfig {
    #[serde(flatten)]
    pub location: MountableDeviceLocation,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Hash)]
#[serde(deny_unknown_fields)]
pub struct MassStorageConfig {
    pub name: String,
    #[serde(flatten)]
    pub location: MountableDeviceLocation,
    pub extensions: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct WebNotificationsConfig {
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct PushoverConfig {
    pub token: String,
    pub recipient: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct SendgridConfig {
    pub token: String,
    pub from: String,
    pub to: String,
    pub subject: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct GoproConfig {
    pub name: String,
    pub serial: String,
}

#[derive(Fail, Debug, PartialEq)]
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
    type Err = ConfigError;

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


        Ok(contents.parse()?)
    }

    /// Get a ConfigBuilder with which you can construct a Config object
    pub fn build() -> ConfigBuilder {
        Default::default()
    }

    /// Serializes this config option into TOML
    pub fn to_toml(&self) -> Result<String, Error> {
        toml::to_string(self).map_err(|e| ConfigError::GenerateError(e).into())
    }

    fn check_config(config: Config) -> Result<Config, ConfigError> {
        if config.dropbox.is_none() && config.vimeo.is_none() {
            Err(ConfigError::MissingBackend)?;
        }

        Config::check_staging(&config.staging)?;

        if let Some(base) = &config.archiver.api_base {
            if let Err(err) = url::Url::parse(&base) {
                Err(ConfigError::InvalidApiBase(err))?;
            }
        }

        Ok(config)
    }

    #[must_use]
    fn check_staging(staging: &StagingConfig) -> Result<(), ConfigError> {
        match &staging.location {
            MountableDeviceLocation::Mountpoint(pb) => {
                if pb.is_relative() {
                    return Err(ConfigError::RelativeStaging.into());
                }
            },
            MountableDeviceLocation::Label(_) => {},
        }
        Ok(())
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

    pub fn notifier(&self) -> Option<Box<dyn Notify>> {
        // Loool
        if let Some(ref web) = self.web_notifications {
            if web.enabled {
                if let Ok(notifier) = WebNotifier::new(&self) {
                    return Some(Box::new(notifier));
                }
            }
        }

        if let Some(ref pshvr) = self.pushover {
            return Some(Box::new(PushoverNotifier::new(
                pshvr.token.clone(),
                pshvr.recipient.clone(),
            )));
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
    pub fn backends(&self) -> Vec<MaybeStorageAdaptor> {
        let mut out = vec![];
        if let Some(ref locals) = self.local_backup {
            for adaptor in locals {
                out.push(match Mountable::mount(adaptor.clone()) {
                    Ok(mounted) => MaybeStorageAdaptor::Ok(mounted),
                    Err(e) => MaybeStorageAdaptor::Err(adaptor.location().to_string(), e),
                });
            }
        }
        if let Some(ref dropbox) = self.dropbox {
            out.push(MaybeStorageAdaptor::Ok(dropbox::DropboxFilesClient::new(dropbox.token.clone())));
        }
        if let Some(ref vimeo) = self.vimeo {
            out.push(MaybeStorageAdaptor::Ok(VimeoClient::new(vimeo.token.clone())));
        }
        out
    }

    /// Returns the configured staging location
    pub fn staging(&self) -> StagingConfig {
        // TODO(richo) This is a bit bizarre, it would kinda be nice to try to guarantee you can
        // only get one copy of staging at a time to avoid trying to mount it twice.
        self.staging.clone()
    }
}

impl ConfigBuilder {
    /// Set the staging directory for this config object
    pub fn staging(mut self, staging: StagingConfig) -> Self {
        self.staging = Some(staging);
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
        let mut local_backups = self.local_backup.unwrap_or_else(|| vec![]);
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

    /// Configure and enable pushover for this config
    // TODO(richo) Should only have one called notifications. Or should both work?
    pub fn web_notifications(mut self) -> Self {
        self.web_notifications = Some(WebNotificationsConfig {
            enabled: true
        });
        self
    }

    /// Configure and enable sendgrid for this config
    pub fn sendgrid(mut self, sendgrid: SendgridConfig) -> Self {
        self.sendgrid = Some(sendgrid);
        self
    }

    /// Finalise this config object
    pub fn finish(self) -> Result<Config, ConfigError> {
        let staging = match self.staging {
            Some(staging) => staging,
            None => return Err(ConfigError::MissingStaging),
        };
        Config::check_config(Config {
            archiver: self.archiver,
            staging: staging,
            dropbox: self.dropbox,
            vimeo: self.vimeo,
            flysight: self.flysight,
            gopro: self.gopro,
            local_backup: self.local_backup,
            mass_storage: self.mass_storage,
            sendgrid: self.sendgrid,
            pushover: self.pushover,
            web_notifications: self.web_notifications,
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
            }
        );

        assert_eq!(
            config.staging,
            StagingConfig {
                location: MountableDeviceLocation::Mountpoint("/test/staging/dir".into()),
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
                location: MountableDeviceLocation::from_mountpoint("/mnt/archiver/flysight".into()),
            }])
        );

        assert_eq!(
            config.mass_storage,
            Some(vec![MassStorageConfig {
                name: "video".into(),
                location: MountableDeviceLocation::from_mountpoint("/mnt/archiver/mass_storage".into()),
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
            config.web_notifications,
            Some(WebNotificationsConfig {
                enabled: true,
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
        let err = Config::from_str(
            r#"
[archiver]
[staging]
mountpoint="test/dir"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap_err();
        assert_eq!(err, ConfigError::RelativeStaging);
    }

    #[test]
    fn test_staging_mountpoint() {
        let cfg = Config::from_str(
            r#"
[archiver]
[staging]
mountpoint="/mnt/staging"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap();
        assert_eq!(cfg.staging,
                   StagingConfig {
                       location: MountableDeviceLocation::Mountpoint("/mnt/staging".into())
                   });
    }

    #[test]
    fn test_staging_label() {
        let cfg = Config::from_str(
            r#"
[archiver]
[staging]
label="STAGING"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap();
        assert_eq!(cfg.staging,
                   StagingConfig {
                       location: MountableDeviceLocation::Label("STAGING".into()),
                   });
    }

    #[test]
    fn test_staging_cannot_be_both() {
        let err = Config::from_str(
            r#"
[archiver]
staging_device="/dev/staging"

[staging]
mountpoint="/test/dir"
label="LABEL"

[dropbox]
token = "TOKEN"
"#,
        ).unwrap_err();
        assert!(match err {
            ConfigError::ParseError(_) => true,
            _ => false,
        });
    }

    #[test]
    fn test_invalid_api_base() {
        let err = Config::from_str(
            r#"
[archiver]
api_base = "malformed"

[staging]
mountpoint = "/test"

[dropbox]
token = "TOKEN"
"#,
        )
        .unwrap_err();
        assert_eq!(ConfigError::InvalidApiBase(url::ParseError::RelativeUrlWithoutBase), err);
    }

    #[test]
    fn test_single_backend() {
        let cfg = Config::from_str(
            r#"
[archiver]
[staging]
mountpoint = "/test"

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
[staging]
mountpoint = "/test"

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
[staging]
mountpoint="/test/dir"

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
[staging]
mountpoint = "/test"
"#,
        )
        .unwrap_err();
        assert_eq!(error, ConfigError::MissingBackend);
    }

    #[test]
    fn test_no_peripherals() {
        let config = Config::from_str(
            r#"
[archiver]
[staging]
mountpoint = "/test"
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
                    location: MountableDeviceLocation::Mountpoint("/mnt/archiver/front".into()),
                    extensions: vec!["mp4".into()],
                },
                MassStorageConfig {
                    name: "back".into(),
                    location: MountableDeviceLocation::Label("back_mass_storage".into()),
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
                    location: MountableDeviceLocation::Mountpoint("/mnt/archiver/training".into()),
                },
                FlysightConfig {
                    name: "comp".into(),
                    location: MountableDeviceLocation::Label("COMP_FLYSIGHT".into()),
                }
            ]
        )
    }

    #[test]
    fn test_mass_storages() {
        let config = Config::from_str(
            r#"
[archiver]
[staging]
mountpoint = "/test"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[mass_storage]]
name = "front"
mountpoint="/mnt/archiver/front"
extensions = ["mp4"]

[[mass_storage]]
name = "back"
label = "back_mass_storage"
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
[staging]
mountpoint = "/test"
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
[staging]
mountpoint = "/test"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[flysight]]
name = "training"
mountpoint="/mnt/archiver/training"

[[flysight]]
name = "comp"
label="COMP_FLYSIGHT"
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
[staging]
mountpoint = "/test"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[mass_storage]]
name = "front"
mountpoint="/mnt/archiver/front"
extensions = ["mp4"]

[[mass_storage]]
name = "back"
label="back_mass_storage"
extensions = ["mov"]

[[flysight]]
name = "training"
mountpoint="/mnt/archiver/training"

[[flysight]]
name = "comp"
label="COMP_FLYSIGHT"
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
