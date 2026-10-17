use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use crate::config::get_config_dir;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HistoryEntry {
    pub id: i64,
    pub tmdb_id: i64,
    pub media_type: String,
    pub title: String,
    pub year: String,
    pub poster_path: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub ep_title: Option<String>,
    pub progress_seconds: i64,
    pub duration_seconds: i64,
    pub last_watched: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FavoriteEntry {
    pub tmdb_id: i64,
    pub media_type: String,
    pub title: String,
    pub year: String,
    pub poster_path: String,
    pub rating: f64,
    pub overview: String,
    pub added_at: String,
}

#[derive(Clone)]
pub struct Database {
    db_path: PathBuf,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = get_config_dir().join("kittymov.db");
        if !db_path.exists() {
            if let Some(parent) = get_config_dir().parent() {
                let old_db = parent.join("cachyflix").join("cachyflix.db");
                if old_db.exists() {
                    let _ = std::fs::copy(old_db, &db_path);
                }
            }
        }
        let s = Self { db_path };
        s.init_tables()?;
        Ok(s)
    }

    fn conn(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tmdb_id INTEGER NOT NULL,
                media_type TEXT NOT NULL,
                title TEXT NOT NULL,
                year TEXT,
                poster_path TEXT,
                season INTEGER,
                episode INTEGER,
                ep_title TEXT,
                progress_seconds INTEGER DEFAULT 0,
                duration_seconds INTEGER DEFAULT 0,
                last_watched TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tmdb_id, media_type, season, episode)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS favorites (
                tmdb_id INTEGER NOT NULL,
                media_type TEXT NOT NULL,
                title TEXT NOT NULL,
                year TEXT,
                poster_path TEXT,
                rating REAL DEFAULT 0.0,
                overview TEXT,
                added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(tmdb_id, media_type)
            )",
            [],
        )?;
        Ok(())
    }

    pub fn record_watch(
        &self,
        tmdb_id: i64,
        media_type: &str,
        title: &str,
        year: &str,
        poster_path: &str,
        season: Option<i32>,
        episode: Option<i32>,
        ep_title: Option<&str>,
        progress_seconds: i64,
        duration_seconds: i64,
    ) -> Result<()> {
        let conn = self.conn()?;
        let s_val = season.unwrap_or(0);
        let e_val = episode.unwrap_or(0);
        conn.execute(
            "INSERT INTO history (
                tmdb_id, media_type, title, year, poster_path, season, episode, ep_title, progress_seconds, duration_seconds, last_watched
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, CURRENT_TIMESTAMP)
            ON CONFLICT(tmdb_id, media_type, season, episode) DO UPDATE SET
                title = excluded.title,
                year = excluded.year,
                poster_path = excluded.poster_path,
                ep_title = excluded.ep_title,
                progress_seconds = excluded.progress_seconds,
                duration_seconds = excluded.duration_seconds,
                last_watched = CURRENT_TIMESTAMP",
            params![
                tmdb_id,
                media_type,
                title,
                year,
                poster_path,
                s_val,
                e_val,
                ep_title.unwrap_or(""),
                progress_seconds,
                duration_seconds
            ],
        )?;
        Ok(())
    }

    pub fn get_history(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, tmdb_id, media_type, title, year, poster_path, season, episode, ep_title, progress_seconds, duration_seconds, last_watched 
             FROM history ORDER BY last_watched DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map([limit as i64], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                tmdb_id: row.get(1)?,
                media_type: row.get(2)?,
                title: row.get(3)?,
                year: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                poster_path: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                season: row.get(6)?,
                episode: row.get(7)?,
                ep_title: row.get(8)?,
                progress_seconds: row.get(9)?,
                duration_seconds: row.get(10)?,
                last_watched: row.get(11)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_latest_watch(&self) -> Result<Option<HistoryEntry>> {
        let list = self.get_history(1)?;
        Ok(list.into_iter().next())
    }

    pub fn toggle_favorite(
        &self,
        tmdb_id: i64,
        media_type: &str,
        title: &str,
        year: &str,
        poster_path: &str,
        rating: f64,
        overview: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorites WHERE tmdb_id = ?1 AND media_type = ?2)",
            params![tmdb_id, media_type],
            |row| row.get(0),
        )?;

        if exists {
            conn.execute(
                "DELETE FROM favorites WHERE tmdb_id = ?1 AND media_type = ?2",
                params![tmdb_id, media_type],
            )?;
            Ok(false)
        } else {
            conn.execute(
                "INSERT INTO favorites (tmdb_id, media_type, title, year, poster_path, rating, overview)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![tmdb_id, media_type, title, year, poster_path, rating, overview],
            )?;
            Ok(true)
        }
    }

    pub fn is_favorite(&self, tmdb_id: i64, media_type: &str) -> bool {
        if let Ok(conn) = self.conn() {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM favorites WHERE tmdb_id = ?1 AND media_type = ?2)",
                params![tmdb_id, media_type],
                |row| row.get(0),
            ).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn get_favorites(&self) -> Result<Vec<FavoriteEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT tmdb_id, media_type, title, year, poster_path, rating, overview, added_at 
             FROM favorites ORDER BY added_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(FavoriteEntry {
                tmdb_id: row.get(0)?,
                media_type: row.get(1)?,
                title: row.get(2)?,
                year: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                poster_path: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                rating: row.get(5)?,
                overview: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                added_at: row.get(7)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}
