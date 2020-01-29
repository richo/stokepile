use crate::web::models::{Equipment, Component};

#[derive(Debug, Serialize)]
pub struct Assembly {
    pub equipment: Equipment,
    pub components: Vec<Component>,
}
