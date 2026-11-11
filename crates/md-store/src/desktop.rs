use super::*;
impl Store {
    pub fn menu_bar_only(&self) -> rusqlite::Result<bool> {
        use rusqlite::OptionalExtension;
        Ok(self
            .0
            .query_row(
                "SELECT value FROM settings WHERE key='menu-bar-only'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .is_some_and(|s| s == "on"))
    }
    pub fn set_menu_bar_only(&self, on: bool) -> rusqlite::Result<()> {
        self.0.execute("INSERT INTO settings VALUES('menu-bar-only',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[if on{"on"}else{"off"}])?;
        Ok(())
    }
}

impl Store {
    pub fn automatic_updates(&self) -> rusqlite::Result<bool> {
        use rusqlite::OptionalExtension;
        Ok(self
            .0
            .query_row(
                "SELECT value FROM settings WHERE key='automatic-updates'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .is_none_or(|v| v != "off"))
    }
    pub fn set_automatic_updates(&self, enabled: bool) -> rusqlite::Result<()> {
        self.0.execute("INSERT INTO settings VALUES('automatic-updates',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [if enabled { "on" } else { "off" }])?;
        Ok(())
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_opt_out_survives_restart() {
        let dir = std::env::temp_dir().join(format!(
            "moodburo-updates-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.db");
        {
            let store = Store::open(&path).unwrap();
            assert!(store.automatic_updates().unwrap());
            store.set_automatic_updates(false).unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert!(!store.automatic_updates().unwrap());
        store.set_automatic_updates(true).unwrap();
        assert!(store.automatic_updates().unwrap());
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
