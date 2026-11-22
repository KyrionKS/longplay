//! SQLite cache for the library index.
//!
//! Albums are stored with their sides as JSON: the shape is small, stable and
//! only ever read as a whole, so normalizing tracks would buy nothing.

use std::path::Path;

use rusqlite::{Connection, params};

use super::model::{Album, Palette};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self, DbError> {
        let db = Self {
            conn: Connection::open_in_memory()?,
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS albums (
                 id           TEXT PRIMARY KEY,
                 title        TEXT NOT NULL,
                 album_artist TEXT NOT NULL,
                 year         INTEGER,
                 genre        TEXT,
                 label        TEXT,
                 added_at     INTEGER NOT NULL,
                 cover        TEXT,
                 palette      TEXT NOT NULL,
                 format       TEXT NOT NULL,
                 sides        TEXT NOT NULL,
                 folder       TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_albums_artist ON albums(album_artist COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_albums_added ON albums(added_at DESC);
             CREATE TABLE IF NOT EXISTS folders (
                 path     TEXT PRIMARY KEY,
                 added_at INTEGER NOT NULL
             );
             PRAGMA user_version=1;",
        )?;
        Ok(())
    }

    pub fn replace_albums(&mut self, albums: &[Album]) -> Result<(), DbError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM albums", [])?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO albums
                 (id, title, album_artist, year, genre, label, added_at, cover, palette, format, sides, folder)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )?;
            for album in albums {
                insert.execute(params![
                    album.id,
                    album.title,
                    album.album_artist,
                    album.year,
                    album.genre,
                    album.label,
                    album.added_at,
                    album.cover,
                    serde_json::to_string(&album.palette)?,
                    serde_json::to_string(&album.format)?,
                    serde_json::to_string(&album.sides)?,
                    album.folder,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_albums(&self) -> Result<Vec<Album>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, album_artist, year, genre, label, added_at, cover, palette, format, sides, folder
             FROM albums ORDER BY added_at DESC, album_artist COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            let palette: String = row.get(8)?;
            let format: String = row.get(9)?;
            let sides: String = row.get(10)?;
            Ok(Album {
                id: row.get(0)?,
                title: row.get(1)?,
                album_artist: row.get(2)?,
                year: row.get(3)?,
                genre: row.get(4)?,
                label: row.get(5)?,
                added_at: row.get(6)?,
                cover: row.get(7)?,
                palette: serde_json::from_str(&palette).unwrap_or_else(|_| Palette::fallback()),
                format: serde_json::from_str(&format).unwrap_or(super::model::DiscFormat::Unknown),
                sides: serde_json::from_str(&sides).unwrap_or_default(),
                folder: row.get(11)?,
                track_count: 0,
                duration: 0.0,
            })
        })?;

        let mut albums = Vec::new();
        for row in rows {
            let mut album = row?;
            album.track_count = album.total_tracks();
            album.duration = album
                .sides
                .iter()
                .flat_map(|side| &side.tracks)
                .map(|track| track.duration + track.pregap)
                .sum();
            albums.push(album);
        }
        Ok(albums)
    }

    pub fn album(&self, id: &str) -> Result<Option<Album>, DbError> {
        Ok(self.load_albums()?.into_iter().find(|album| album.id == id))
    }

    pub fn set_folders(&mut self, folders: &[std::path::PathBuf]) -> Result<(), DbError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM folders", [])?;
        {
            let mut insert = tx.prepare("INSERT INTO folders (path, added_at) VALUES (?1, ?2)")?;
            let now = now_secs();
            for folder in folders {
                insert.execute(params![folder.to_string_lossy(), now])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn folders(&self) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM folders ORDER BY added_at")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::model::{DiscFormat, Rpm, Side, Track};

    fn album(id: &str) -> Album {
        Album {
            id: id.to_string(),
            title: "Low Tide Hymns".into(),
            album_artist: "Marlowe & the Drift".into(),
            year: Some(1972),
            genre: Some("Folk".into()),
            label: Some("Blue Lamp".into()),
            added_at: 1_700_000_000,
            cover: None,
            palette: Palette::fallback(),
            format: DiscFormat::Lp12,
            sides: vec![Side {
                label: "A".into(),
                rpm: Rpm::R33,
                matrix: Some("LP-001-A".into()),
                tracks: vec![Track {
                    id: "t1".into(),
                    number: 1,
                    title: "Under the Broken Motel".into(),
                    artist: "Marlowe & the Drift".into(),
                    duration: 149.0,
                    pregap: 1.5,
                    path: "/music/album.flac".into(),
                    offset: 0.0,
                    side: "A".into(),
                }],
            }],
            folder: "/music".into(),
            track_count: 0,
            duration: 0.0,
        }
    }

    #[test]
    fn round_trips_albums() {
        let mut db = Db::in_memory().unwrap();
        db.replace_albums(&[album("one")]).unwrap();

        let loaded = db.load_albums().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].title, "Low Tide Hymns");
        assert_eq!(loaded[0].track_count, 1);
        assert!((loaded[0].duration - 150.5).abs() < 1e-9);
        assert_eq!(loaded[0].sides[0].rpm, Rpm::R33);
    }

    #[test]
    fn replaces_instead_of_accumulating() {
        let mut db = Db::in_memory().unwrap();
        db.replace_albums(&[album("one"), album("two")]).unwrap();
        db.replace_albums(&[album("two")]).unwrap();
        let loaded = db.load_albums().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "two");
    }

    #[test]
    fn stores_scan_folders() {
        let mut db = Db::in_memory().unwrap();
        db.set_folders(&[
            std::path::PathBuf::from("/music/a"),
            std::path::PathBuf::from("/music/b"),
        ])
        .unwrap();
        assert_eq!(db.folders().unwrap(), vec!["/music/a", "/music/b"]);
    }
}
