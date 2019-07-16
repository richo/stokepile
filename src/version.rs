pub static VERSION: &'static str = "0.2.0";
pub static GIT_HASH: Option<&'static str> = option_env!("GIT_HASH");
pub static AUTHOR: &'static str = "richö butts";

#[cfg(test)]
mod tests {
    use toml;

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
