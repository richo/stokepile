use stokepile;
use toml;
use std::io::Read;
use std::fs::File;
use std::path::PathBuf;

fn get_version(path: PathBuf) -> String {
    let mut fh = File::open(path).unwrap();
    let mut contents = String::new();
    fh.read_to_string(&mut contents).unwrap();

    let config = contents.parse::<toml::Value>().unwrap();
    config["package"]["version"].as_str().expect("missing key or wrong type").into()

}

#[test]
fn test_wasm_version_matches() {
    assert_eq!(stokepile::VERSION, get_version("wasm/Cargo.toml".into()));
}

#[test]
fn test_shared_version_matches() {
    assert_eq!(stokepile::VERSION, get_version("shared/Cargo.toml".into()));
}
