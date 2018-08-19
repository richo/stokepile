#[allow(non_camel_case_types)]
#[derive(Deserialize,Debug,Eq,PartialEq)]
enum StorageBackend {
    dropbox,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
struct Config {
    archiver: ArchiverConfig,
    dropbox: Option<DropboxConfig>,
    // vimeo: Option<VimeoConfig>,
    // youtube: Option<YoutubeConfig>,
    flysight: Vec<FlysightConfig>,
    gopro: Vec<GoproConfig>,
    // gswoop: Option<GswoopConfig>,
    // sendgrid: Option<SendgridConfig>,
    // pushover: Option<PushoverConfig>,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
struct ArchiverConfig {
    storage_backend: StorageBackend,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
struct DropboxConfig {
    token: String,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
struct FlysightConfig {
    name: String,
    mountpoint: Option<String>,
    uuid: Option<String>,
}

#[derive(Deserialize,Debug,Eq,PartialEq)]
struct GoproConfig {
    name: String,
    mountpoint: Option<String>,
    uuid: Option<String>,
}

#[cfg(test)]
mod tests {
    extern crate toml;

    use super::*;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test_example_config_parses() {
        let mut fh = File::open("archiver.toml.example").unwrap();
        let mut contents = String::new();
        fh.read_to_string(&mut contents).unwrap();

        let config: Config = toml::from_str(&contents).unwrap();

        assert_eq!(config.archiver.storage_backend, StorageBackend::dropbox);

        assert_eq!(config.dropbox,
                   Some(DropboxConfig{ token: "DROPBOX_TOKEN_GOES_HERE".into() }));

        assert_eq!(config.flysight,
                   vec![FlysightConfig {
                            name: "data".into(),
                            mountpoint: Some("/mnt/archiver/flysight".into()),
                            uuid: None,
                   }]);

        assert_eq!(config.gopro,
                   vec![GoproConfig {
                            name: "video".into(),
                            mountpoint: Some("/mnt/archiver/gopro".into()),
                            uuid: None,
                   }]);
    }
}
