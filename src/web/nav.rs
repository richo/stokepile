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
            NavEntry { name: "Home", active: false, location: "/" },
            NavEntry { name: "Link Dropbox", active: false, location: "/dropbox/auth" },
            NavEntry { name: "Link Youtube", active: false, location: "/youtube/auth" },
            NavEntry { name: "Download Config", active: false, location: "/config.json" },
        ]
    };
}
