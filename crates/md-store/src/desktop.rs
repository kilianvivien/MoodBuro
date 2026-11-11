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
