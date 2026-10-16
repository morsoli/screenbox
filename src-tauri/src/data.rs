use crate::model::{media_tool, CursorPoint, EditDocument, Project};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;
use std::process::Command;
use tauri::State;

use crate::settings::UserPreferences;

pub struct Database {
    pub root: PathBuf,
    path: PathBuf,
}

impl Database {
    pub fn open(root: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let path = root.join("screenbox.sqlite3");
        let db = Self { root, path };
        let conn = db.conn()?;
        conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY, title TEXT NOT NULL, original_path TEXT NOT NULL, cursor_path TEXT NOT NULL,
            duration_ms INTEGER NOT NULL, width INTEGER NOT NULL, height INTEGER NOT NULL, microphone INTEGER NOT NULL,
            created_at TEXT NOT NULL, edit_json TEXT NOT NULL, last_export_path TEXT
        ); CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
        drop(conn);
        db.recover_orphaned_recordings();
        Ok(db)
    }
    pub fn conn(&self) -> rusqlite::Result<Connection> {
        Connection::open(&self.path)
    }
    pub fn insert(&self, project: &Project) -> Result<(), String> {
        self.conn()
            .map_err(|e| e.to_string())?
            .execute(
                "INSERT INTO projects VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    project.id,
                    project.title,
                    project.original_path,
                    project.cursor_path,
                    project.duration_ms,
                    project.width,
                    project.height,
                    project.microphone,
                    project.created_at,
                    serde_json::to_string(&project.edit).unwrap(),
                    project.last_export_path
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn get(&self, id: &str) -> Result<Project, String> {
        let conn = self.conn().map_err(|e| e.to_string())?;
        conn.query_row("SELECT id,title,original_path,cursor_path,duration_ms,width,height,microphone,created_at,edit_json,last_export_path FROM projects WHERE id=?1", [id], row_project)
            .map_err(|e| e.to_string())
    }
    pub fn list(&self) -> Result<Vec<Project>, String> {
        let conn = self.conn().map_err(|e| e.to_string())?;
        let mut statement = conn.prepare("SELECT id,title,original_path,cursor_path,duration_ms,width,height,microphone,created_at,edit_json,last_export_path FROM projects ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], row_project)
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }
    pub fn update_edit(&self, id: &str, edit: &EditDocument) -> Result<(), String> {
        let project = self.get(id)?;
        edit.validate(project.duration_ms)?;
        self.conn()
            .map_err(|e| e.to_string())?
            .execute(
                "UPDATE projects SET edit_json=?1 WHERE id=?2",
                params![serde_json::to_string(edit).unwrap(), id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn set_export(&self, id: &str, path: &str) -> Result<(), String> {
        self.conn()
            .map_err(|e| e.to_string())?
            .execute(
                "UPDATE projects SET last_export_path=?1 WHERE id=?2",
                params![path, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn preferences(&self) -> Result<UserPreferences, String> {
        let conn = self.conn().map_err(|e| e.to_string())?;
        let read = |key: &str| -> Result<Option<String>, String> {
            conn.query_row("SELECT value FROM settings WHERE key=?1", [key], |row| row.get(0))
                .optional()
                .map_err(|e| e.to_string())
        };
        let mut preferences = UserPreferences::default();
        preferences.language = read("language")?
            .and_then(|value| serde_json::from_str(&format!("\"{value}\"")).ok())
            .unwrap_or_default();
        preferences.theme = read("theme")?
            .and_then(|value| serde_json::from_str(&format!("\"{value}\"")).ok())
            .unwrap_or_default();
        Ok(preferences)
    }

    pub fn save_preferences(&self, preferences: &UserPreferences) -> Result<(), String> {
        let mut conn = self.conn().map_err(|e| e.to_string())?;
        let transaction = conn.transaction().map_err(|e| e.to_string())?;
        transaction.execute("INSERT INTO settings(key,value) VALUES('language',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [preferences.language.as_str()])
            .map_err(|e| e.to_string())?;
        transaction.execute("INSERT INTO settings(key,value) VALUES('theme',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [preferences.theme.as_str()])
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn recover_orphaned_recordings(&self) {
        let recordings = self.root.join("recordings");
        let Ok(entries) = std::fs::read_dir(recordings) else { return };
        for entry in entries.flatten() {
            let folder = entry.path();
            let Some(id) = folder.file_name().and_then(|name| name.to_str()) else { continue };
            if uuid::Uuid::parse_str(id).is_err() { continue; }
            let original = folder.join("original.mp4");
            if !original.is_file() || std::fs::metadata(&original).map(|m| m.len() < 1024).unwrap_or(true) { continue; }
            let already_indexed = self.conn().ok().and_then(|conn| {
                conn.query_row("SELECT 1 FROM projects WHERE id=?1", [id], |_| Ok(())).ok()
            }).is_some();
            if already_indexed { continue; }
            let Some((duration_ms, width, height, microphone)) = inspect_video(&original) else { continue };
            let cursor = folder.join("cursor.jsonl");
            let created_at = std::fs::metadata(&original).ok().and_then(|m| m.modified().ok())
                .map(chrono::DateTime::<chrono::Utc>::from).unwrap_or_else(chrono::Utc::now).to_rfc3339();
            let project = Project {
                id: id.to_string(), title: format!("Recovered recording {}", &id[..8]),
                original_path: original.to_string_lossy().to_string(), cursor_path: cursor.to_string_lossy().to_string(),
                duration_ms, width, height, microphone, created_at,
                edit: EditDocument::default_for(duration_ms), last_export_path: None,
            };
            if self.insert(&project).is_ok() {
                let poster = folder.join("original.poster.jpg");
                if !poster.exists() {
                    let _ = Command::new(media_tool("ffmpeg"))
                        .args(["-hide_banner", "-loglevel", "error", "-y", "-ss", "0.1", "-i"])
                        .arg(&original).args(["-frames:v", "1", "-q:v", "3"]).arg(poster).status();
                }
            }
        }
    }
}

fn inspect_video(path: &std::path::Path) -> Option<(u64, u32, u32, bool)> {
    let output = Command::new(media_tool("ffmpeg")).args(["-hide_banner", "-i"]).arg(path).output().ok()?;
    let details = String::from_utf8_lossy(&output.stderr);
    let duration = details.split("Duration:").nth(1)?.split(',').next()?.trim();
    let mut time = duration.split(':');
    let hours = time.next()?.parse::<f64>().ok()?;
    let minutes = time.next()?.parse::<f64>().ok()?;
    let seconds = time.next()?.parse::<f64>().ok()?;
    let duration_ms = ((hours * 3600.0 + minutes * 60.0 + seconds) * 1000.0) as u64;
    let (width, height) = details.split_whitespace().find_map(|word| {
        let (width, height) = word.trim_matches(|c: char| !c.is_ascii_digit() && c != 'x').split_once('x')?;
        let width = width.parse::<u32>().ok()?;
        let height = height.parse::<u32>().ok()?;
        (width > 0 && height > 0).then_some((width, height))
    })?;
    (duration_ms > 0).then_some((duration_ms, width, height, details.contains("Audio:")))
}

fn row_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    let edit_json: String = row.get(9)?;
    let edit = serde_json::from_str(&edit_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Project {
        id: row.get(0)?,
        title: row.get(1)?,
        original_path: row.get(2)?,
        cursor_path: row.get(3)?,
        duration_ms: row.get(4)?,
        width: row.get(5)?,
        height: row.get(6)?,
        microphone: row.get(7)?,
        created_at: row.get(8)?,
        edit,
        last_export_path: row.get(10)?,
    })
}

#[cfg(test)]
mod preference_tests {
    use super::*;
    use crate::settings::{LanguagePreference, ThemePreference};

    #[test]
    fn preferences_default_and_persist_across_database_reopen() {
        let root = std::env::temp_dir().join(format!("screenbox-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::open(root.clone()).unwrap();
        assert_eq!(db.preferences().unwrap(), UserPreferences::default());
        let selected = UserPreferences { language: LanguagePreference::Chinese, theme: ThemePreference::Dark };
        db.save_preferences(&selected).unwrap();
        drop(db);
        let reopened = Database::open(root.clone()).unwrap();
        assert_eq!(reopened.preferences().unwrap(), selected);
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[tauri::command]
pub fn list_projects(db: State<'_, Database>) -> Result<Vec<Project>, String> {
    db.list()
}

#[tauri::command]
pub fn save_edit(db: State<'_, Database>, id: String, edit: EditDocument) -> Result<(), String> {
    db.update_edit(&id, &edit)
}

#[tauri::command]
pub fn cursor_track(db: State<'_, Database>, id: String) -> Result<Vec<CursorPoint>, String> {
    let project = db.get(&id)?;
    let points = std::fs::read_to_string(project.cursor_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<CursorPoint>(line).ok())
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .take(100_000)
        .collect();
    Ok(points)
}

#[tauri::command]
pub fn get_preferences(db: State<'_, Database>) -> Result<UserPreferences, String> {
    db.preferences()
}
