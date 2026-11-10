use super::{params, Store};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bookmark {
    pub id: String,
    pub site_id: String,
    pub user_id: i64,
    pub name: String,
    pub path: String,
    pub icon: String,
}
impl Store {
    pub fn bookmarks(&self) -> rusqlite::Result<Vec<Bookmark>> {
        let mut stmt = self.0.prepare("SELECT b.id,b.site_id,b.user_id,b.name,b.path,b.icon FROM bookmarks b JOIN sites s ON s.id=b.site_id AND s.user_id=b.user_id ORDER BY b.rowid")?;
        let rows = stmt.query_map([], |r| {
            Ok(Bookmark {
                id: r.get(0)?,
                site_id: r.get(1)?,
                user_id: r.get(2)?,
                name: r.get(3)?,
                path: r.get(4)?,
                icon: r.get(5)?,
            })
        })?;
        rows.collect()
    }
    pub fn bookmark_save(&self, b: &Bookmark) -> rusqlite::Result<()> {
        let changed = self.0.execute("INSERT INTO bookmarks(id,site_id,user_id,name,path,icon) SELECT ?1,?2,?3,?4,?5,?6 WHERE EXISTS(SELECT 1 FROM sites WHERE id=?2 AND user_id=?3) ON CONFLICT(id) DO UPDATE SET name=excluded.name,icon=excluded.icon WHERE bookmarks.site_id=excluded.site_id AND bookmarks.user_id=excluded.user_id AND bookmarks.path=excluded.path", params![b.id,b.site_id,b.user_id,b.name,b.path,b.icon])?;
        if changed == 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(())
    }
    pub fn bookmark_delete(&self, id: &str) -> rusqlite::Result<()> {
        self.0.execute("DELETE FROM bookmarks WHERE id=?1", [id])?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bookmarks_are_persistent_ordered_and_account_bound() {
        let path =
            std::env::temp_dir().join(format!("moodburo-bookmarks-{}.db", std::process::id()));
        let s = Store::open(&path).unwrap();
        let mut site = crate::Site {
            id: "one".into(),
            wwwroot: "https://example.org/".into(),
            name: "Test".into(),
            user_id: 1,
            full_name: "Test".into(),
        };
        s.insert(&site).unwrap();
        let mut b = Bookmark {
            id: "first".into(),
            site_id: site.id.clone(),
            user_id: 1,
            name: "Course".into(),
            path: "course/view.php?id=42".into(),
            icon: "book".into(),
        };
        s.bookmark_save(&b).unwrap();
        b.id = "second".into();
        s.bookmark_save(&b).unwrap();
        b.id = "first".into();
        b.name = "Renamed".into();
        b.icon = "star".into();
        s.bookmark_save(&b).unwrap();
        drop(s);
        let s = Store::open(&path).unwrap();
        let list = s.bookmarks().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Renamed");
        assert_eq!(list[0].icon, "star");
        b.user_id = 2;
        assert!(s.bookmark_save(&b).is_err());
        s.bookmark_delete("second").unwrap();
        assert_eq!(s.bookmarks().unwrap().len(), 1);
        site.user_id = 2;
        s.update(&site).unwrap();
        assert!(s.bookmarks().unwrap().is_empty());
        b.user_id = 2;
        s.bookmark_save(&b).unwrap();
        s.remove(&site.id).unwrap();
        assert!(s.bookmarks().unwrap().is_empty());
        drop(s);
        let _ = std::fs::remove_file(path);
    }
}
