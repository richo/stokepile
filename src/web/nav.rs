#[derive(Serialize, Clone)]
pub(in web) struct NavEntry {
    pub name: &'static str,
    pub active: bool,
    pub location: &'static str,
}

#[derive(Serialize, Clone)]
pub struct NavMap {
    pub(in web) routes: Vec<NavEntry>,
}

lazy_static! {
    pub static ref NAV_MAP: NavMap = NavMap {
        routes: vec![
            NavEntry { name: "index", active: false, location: "/" },
            NavEntry { name: "authorize_dropbox", active: false, location: "/authorize/dropbox" },
        ]
    };
}
