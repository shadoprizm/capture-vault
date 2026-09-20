use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;
use uuid::Uuid;

use crate::models::{CaptureMode, CaptureRecord};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Could not access CaptureVault storage: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not update the CaptureVault database: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Could not read screenshot dimensions: {0}")]
    Image(#[from] image::ImageError),
    #[error("Could not encode screenshot tags: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Capture {0} was not found")]
    NotFound(String),
}

#[derive(Debug)]
pub struct CaptureStore {
    root: PathBuf,
    database_path: PathBuf,
    captures_dir: PathBuf,
}

impl CaptureStore {
    pub fn new(root: PathBuf) -> Result<Self, StorageError> {
        let captures_dir = root.join("captures");
        fs::create_dir_all(&captures_dir)?;

        let store = Self {
            database_path: root.join("capture-vault.sqlite3"),
            root,
            captures_dir,
        };
        store.initialize()?;
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, StorageError> {
        let connection = Connection::open(&self.database_path)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(connection)
    }

    fn initialize(&self) -> Result<(), StorageError> {
        let connection = self.connection()?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS captures (
                id TEXT PRIMARY KEY NOT NULL,
                created_at TEXT NOT NULL,
                relative_path TEXT NOT NULL UNIQUE,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                capture_mode TEXT NOT NULL,
                note TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                favorite INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS captures_created_at_idx
                ON captures(created_at DESC);
            PRAGMA user_version = 1;",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<CaptureRecord>, StorageError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, created_at, relative_path, width, height,
                    capture_mode, note, tags_json, favorite
             FROM captures
             ORDER BY created_at DESC",
        )?;

        let rows = statement.query_map([], |row| self.record_from_row(row))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn import_capture(
        &self,
        source: &Path,
        mode: CaptureMode,
    ) -> Result<CaptureRecord, StorageError> {
        let (width, height) = image::image_dimensions(source)?;
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let filename = format!("capture-{timestamp}-{}.png", &id[..8]);
        let relative_path = Path::new("captures").join(filename);
        let destination = self.root.join(&relative_path);
        let temporary = destination.with_extension("png.part");

        fs::copy(source, &temporary)?;
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }

        let connection = self.connection()?;
        if let Err(error) = connection.execute(
            "INSERT INTO captures (
                id, created_at, relative_path, width, height, capture_mode,
                note, tags_json, favorite
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', '[]', 0)",
            params![
                id,
                created_at,
                relative_path.to_string_lossy(),
                width,
                height,
                mode.as_str()
            ],
        ) {
            let _ = fs::remove_file(&destination);
            return Err(error.into());
        }

        self.get(&id)
    }

    pub fn update_metadata(
        &self,
        id: &str,
        note: &str,
        tags: &[String],
        favorite: bool,
    ) -> Result<CaptureRecord, StorageError> {
        let mut seen = HashSet::new();
        let clean_tags: Vec<String> = tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .filter(|tag| seen.insert(tag.to_lowercase()))
            .take(20)
            .map(|tag| tag.chars().take(40).collect())
            .collect();
        let tags_json = serde_json::to_string(&clean_tags)?;
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE captures
             SET note = ?2, tags_json = ?3, favorite = ?4
             WHERE id = ?1",
            params![id, note.trim(), tags_json, favorite],
        )?;

        if changed == 0 {
            return Err(StorageError::NotFound(id.to_owned()));
        }

        self.get(id)
    }

    pub fn delete(&self, id: &str) -> Result<(), StorageError> {
        let capture = self.get(id)?;
        let original = PathBuf::from(&capture.file_path);
        let staged = self.captures_dir.join(format!(".{id}.deleting"));

        if original.exists() {
            fs::rename(&original, &staged)?;
        }

        let connection = self.connection()?;
        if let Err(error) = connection.execute("DELETE FROM captures WHERE id = ?1", [id]) {
            if staged.exists() {
                let _ = fs::rename(&staged, &original);
            }
            return Err(error.into());
        }

        if staged.exists() {
            fs::remove_file(staged)?;
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<CaptureRecord, StorageError> {
        let connection = self.connection()?;
        let record = connection
            .query_row(
                "SELECT id, created_at, relative_path, width, height,
                        capture_mode, note, tags_json, favorite
                 FROM captures
                 WHERE id = ?1",
                [id],
                |row| self.record_from_row(row),
            )
            .optional()?;

        record.ok_or_else(|| StorageError::NotFound(id.to_owned()))
    }

    fn record_from_row(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<CaptureRecord> {
        let relative_path: String = row.get(2)?;
        let tags_json: String = row.get(7)?;
        let tags = serde_json::from_str(&tags_json).unwrap_or_default();

        Ok(CaptureRecord {
            id: row.get(0)?,
            created_at: row.get(1)?,
            file_path: self.root.join(relative_path).to_string_lossy().into_owned(),
            width: row.get(3)?,
            height: row.get(4)?,
            capture_mode: row.get(5)?,
            note: row.get(6)?,
            tags,
            favorite: row.get::<_, i64>(8)? != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, Rgba};
    use tempfile::tempdir;

    use super::*;

    fn sample_png(path: &Path) {
        ImageBuffer::from_pixel(12, 8, Rgba([28_u8, 36, 31, 255]))
            .save(path)
            .expect("write sample image");
    }

    #[test]
    fn imports_updates_and_deletes_a_capture() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("source.png");
        sample_png(&source);
        let store = CaptureStore::new(directory.path().join("library")).expect("create store");

        let capture = store
            .import_capture(&source, CaptureMode::Area)
            .expect("import capture");
        assert_eq!((capture.width, capture.height), (12, 8));
        assert_eq!(store.list().expect("list captures").len(), 1);

        let updated = store
            .update_metadata(
                &capture.id,
                "Reference image",
                &[" Design ".into(), "design".into(), "Ubuntu".into()],
                true,
            )
            .expect("update metadata");
        assert_eq!(updated.tags, vec!["Design", "Ubuntu"]);
        assert!(updated.favorite);

        store.delete(&capture.id).expect("delete capture");
        assert!(store.list().expect("list captures").is_empty());
        assert!(!Path::new(&capture.file_path).exists());
    }
}
