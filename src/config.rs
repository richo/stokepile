extern crate toml;
extern crate failure;

use std::fs::File;
use std::io::Read;

use failure::Error;


#[allow(non_camel_case_types)]
#[derive(Deserialize,Debug,Eq,PartialEq)]
pub enum StorageBackend {
    dropbox,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
pub struct Config {
    archiver: ArchiverConfig,
    dropbox: DropboxConfig,
    // vimeo: Option<VimeoConfig>,
    // youtube: Option<YoutubeConfig>,
    flysight: Option<Vec<FlysightConfig>>,
    gopro: Option<Vec<GoproConfig>>,
    // gswoop: Option<GswoopConfig>,
    // sendgrid: Option<SendgridConfig>,
    // pushover: Option<PushoverConfig>,
}

lazy_static! {
    static ref EMPTY_GOPROS: Vec<GoproConfig> = vec![];
    static ref EMPTY_FLYSIGHTS: Vec<FlysightConfig> = vec![];
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
pub struct ArchiverConfig {
    storage_backend: StorageBackend,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
pub struct DropboxConfig {
    token: String,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
pub struct FlysightConfig {
    name: String,
    mountpoint: String,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
pub struct GoproConfig {
    name: String,
    mountpoint: String,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Config, Error> {
        let mut fh = File::open(path)?;
        let mut contents = String::new();
        fh.read_to_string(&mut contents)?;

        Config::from_str(&contents)
    }

    pub fn from_str(body: &str) -> Result<Config, Error> {
        match toml::from_str(body) {
            Ok(config) => Ok(config),
            Err(e) => Err(format_err!("Couldn't parse config: {}", e)),
        }
    }

    // Do we eventually want to make a camera/gopro distinction?
    pub fn gopros(&self) -> &Vec<GoproConfig> {
        match self.gopro {
            None => &EMPTY_GOPROS,
            Some(ref v) => v,
        }
    }

    pub fn flysights(&self) -> &Vec<FlysightConfig> {
        match self.flysight {
            None => &EMPTY_FLYSIGHTS,
            Some(ref v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_config_parses() {
        let config = Config::from_file("archiver.toml.example").unwrap();

        assert_eq!(config.archiver.storage_backend, StorageBackend::dropbox);

        assert_eq!(config.dropbox,
                   DropboxConfig{ token: "DROPBOX_TOKEN_GOES_HERE".into() });

        assert_eq!(config.flysight,
                   Some(vec![FlysightConfig {
                            name: "data".into(),
                            mountpoint: "/mnt/archiver/flysight".into(),
                   }]));

        assert_eq!(config.gopro,
                   Some(vec![GoproConfig {
                            name: "video".into(),
                            mountpoint: "/mnt/archiver/gopro".into(),
                   }]));
    }

    #[test]
    fn test_invalid_backend() {
        let error = Config::from_str(r#"
[archiver]
storage_backend="butts"
"#).unwrap_err();
        assert!(format!("{}", error)
                .contains("unknown variant `butts`, expected `dropbox` for key `archiver.storage_backend`"))
    }

    #[test]
    fn test_no_backend() {
        let error = Config::from_str(r#"
[archiver]
"#).unwrap_err();
        assert!(format!("{}", error)
                .contains("missing field `storage_backend` for key `archiver`"))
    }

    #[test]
    fn test_no_dropbox() {
        let error = Config::from_str(r#"
[archiver]
storage_backend="dropbox"
"#).unwrap_err();
        assert!(format!("{}", error)
                .contains("missing field `dropbox`"))
    }

    #[test]
    fn test_no_peripherals() {
        let config = Config::from_str(r#"
[archiver]
storage_backend="dropbox"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"
"#).unwrap();
        assert_no_gopros(&config);
        assert_no_flysights(&config);
    }

    fn assert_no_gopros(cfg: &Config) {
        assert_eq!(cfg.gopro, None);
        assert_eq!(cfg.gopros(), &vec![]);
    }

    fn assert_no_flysights(cfg: &Config) {
        assert_eq!(cfg.flysight, None);
        assert_eq!(cfg.flysights(), &vec![]);
    }

    fn assert_gopros(cfg: &Config) {
        assert_eq!(cfg.gopros(),
        &vec![GoproConfig {
                name: "front".into(),
                mountpoint: "/mnt/archiver/front".into(),
            },
            GoproConfig {
                name: "back".into(),
                mountpoint: "/mnt/archiver/back".into(),
            }
        ])
    }

    fn assert_flysights(cfg: &Config) {
        assert_eq!(cfg.flysights(),
        &vec![FlysightConfig {
                name: "training".into(),
                mountpoint: "/mnt/archiver/training".into(),
            },
            FlysightConfig {
                name: "comp".into(),
                mountpoint: "/mnt/archiver/comp".into(),
            }
        ])
    }

    #[test]
    fn test_gopros() {
        let config = Config::from_str(r#"
[archiver]
storage_backend="dropbox"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[gopro]]
name = "front"
mountpoint="/mnt/archiver/front"

[[gopro]]
name = "back"
mountpoint="/mnt/archiver/back"
"#).unwrap();
        assert_gopros(&config);
        assert_no_flysights(&config);
    }

    #[test]
    fn test_flysights() {
        let config = Config::from_str(r#"
[archiver]
storage_backend="dropbox"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[flysight]]
name = "training"
mountpoint="/mnt/archiver/training"

[[flysight]]
name = "comp"
mountpoint="/mnt/archiver/comp"
"#).unwrap();
        assert_flysights(&config);
        assert_no_gopros(&config);
    }

    #[test]
    fn test_gopros_and_flysights() {
        let config = Config::from_str(r#"
[archiver]
storage_backend="dropbox"
[dropbox]
token="DROPBOX_TOKEN_GOES_HERE"

[[gopro]]
name = "front"
mountpoint="/mnt/archiver/front"

[[gopro]]
name = "back"
mountpoint="/mnt/archiver/back"

[[flysight]]
name = "training"
mountpoint="/mnt/archiver/training"

[[flysight]]
name = "comp"
mountpoint="/mnt/archiver/comp"
"#).unwrap();
        assert_gopros(&config);
        assert_flysights(&config);
    }
}
