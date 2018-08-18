pub const VERSION: &'static str = "0.1.0";

#[cfg(test)]
mod tests {
    extern crate toml;

    use super::*;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test_versions_all_up_to_date() {
        let mut fh = File::open("Cargo.toml").unwrap();
        let mut contents = String::new();
        fh.read_to_string(&mut contents).unwrap();

        let config = contents.parse::<toml::Value>().unwrap();

        assert_eq!(Some(VERSION), config["package"]["version"].as_str());
    }
}
