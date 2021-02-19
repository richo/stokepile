use diesel::prelude::*;
use chrono::NaiveDate;

use crate::web::models::{Equipment, Component, Repack};

#[derive(Debug, Serialize)]
pub struct Assembly {
    pub equipment: Equipment,
    pub components: Vec<Component>,
    pub last_repack: Option<Repack>,
    pub next_due: Option<NaiveDate>,
}

impl Assembly {
    pub fn repacks(&self, conn: &PgConnection) -> QueryResult<Vec<Repack>> {
        Repack::by_equipment(self.equipment.id, conn)
    }

    pub fn reserve(&self) -> &Component {
        let reserve: Vec<_> = self.components
            .iter()
            .filter(|c| c.kind == "reserve")
            .collect();
        assert_eq!(1, reserve.len());
        return &reserve[0]
    }

    pub fn container(&self) -> &Component {
        let container: Vec<_> = self.components
            .iter()
            .filter(|c| c.kind == "container")
            .collect();
        assert_eq!(1, container.len());
        return &container[0]
    }

    pub fn aad(&self) -> Option<&Component> {
        let mut reserve: Vec<_> = self.components
            .iter()
            .filter(|c| c.kind == "aad")
            .collect();
        return reserve.pop()
    }

    pub fn due_before(&self, date: NaiveDate) -> bool {
        if let Some(due) = self.next_due {
            due < date
        } else {
            true
        }
    }

}
