use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
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
    #[error("This CaptureVault library uses unsupported database version {0}")]
    UnsupportedVersion(i64),
    #[error("Capture {0} was not found")]
    NotFound(String),
    #[error("Choose an absolute folder for screenshot storage")]
    InvalidStorageLocation,
    #[error("The destination already contains a file named {0}")]
    DestinationConflict(String),
    #[error("CaptureVault could not access its active storage location")]
    StorageLock,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageSettings {
    image_directory: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct CaptureStore {
    database_path: PathBuf,
    settings_path: PathBuf,
    captures_dir: Arc<RwLock<PathBuf>>,
    /// Serializes operations that move or mutate capture files. A global
    /// shortcut can invoke `import_capture` while the settings UI migrates the
    /// library, so the active path must not change halfway through an import.
    operation_lock: Arc<Mutex<()>>,
}

impl CaptureStore {
    pub fn new(root: PathBuf) -> Result<Self, StorageError> {
        fs::create_dir_all(&root)?;
        let settings_path = root.join("storage-settings.json");
        let settings = if settings_path.exists() {
            serde_json::from_slice(&fs::read(&settings_path)?)?
        } else {
            StorageSettings::default()
        };
        let captures_dir = settings
            .image_directory
            .unwrap_or_else(|| root.join("captures"));
        fs::create_dir_all(&captures_dir)?;

        let store = Self {
            database_path: root.join("capture-vault.sqlite3"),
            settings_path,
            captures_dir: Arc::new(RwLock::new(captures_dir)),
            operation_lock: Arc::new(Mutex::new(())),
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
        let mut connection = self.connection()?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

        match version {
            0 => connection.execute_batch(
                "CREATE TABLE IF NOT EXISTS captures (
                id TEXT PRIMARY KEY NOT NULL,
                created_at TEXT NOT NULL,
                relative_path TEXT NOT NULL UNIQUE,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                capture_mode TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                favorite INTEGER NOT NULL DEFAULT 0,
                ocr_text TEXT NOT NULL DEFAULT '',
                enrichment_status TEXT NOT NULL DEFAULT 'pending',
                title_edited INTEGER NOT NULL DEFAULT 0,
                description_edited INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS captures_created_at_idx
                ON captures(created_at DESC);
            PRAGMA user_version = 3;",
            )?,
            1 | 2 => {
                let transaction = connection.transaction()?;
                if version == 1 {
                    Self::add_column_if_missing(&transaction, "title", "TEXT NOT NULL DEFAULT ''")?;
                    Self::add_column_if_missing(
                        &transaction,
                        "description",
                        "TEXT NOT NULL DEFAULT ''",
                    )?;
                    Self::add_column_if_missing(
                        &transaction,
                        "ocr_text",
                        "TEXT NOT NULL DEFAULT ''",
                    )?;
                    Self::add_column_if_missing(
                        &transaction,
                        "enrichment_status",
                        "TEXT NOT NULL DEFAULT 'pending'",
                    )?;
                }
                Self::add_column_if_missing(
                    &transaction,
                    "title_edited",
                    "INTEGER NOT NULL DEFAULT 0",
                )?;
                Self::add_column_if_missing(
                    &transaction,
                    "description_edited",
                    "INTEGER NOT NULL DEFAULT 0",
                )?;
                transaction.pragma_update(None, "user_version", 3)?;
                transaction.commit()?;
            }
            3 => {}
            other => return Err(StorageError::UnsupportedVersion(other)),
        }
        Ok(())
    }

    fn add_column_if_missing(
        connection: &Connection,
        column: &str,
        definition: &str,
    ) -> Result<(), StorageError> {
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('captures') WHERE name = ?1)",
            [column],
            |row| row.get(0),
        )?;
        if !exists {
            connection.execute_batch(&format!(
                "ALTER TABLE captures ADD COLUMN {column} {definition};"
            ))?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<CaptureRecord>, StorageError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, created_at, relative_path, width, height,
                    capture_mode, title, description, note, tags_json, favorite,
                    ocr_text, enrichment_status
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
        let _operation = self
            .operation_lock
            .lock()
            .map_err(|_| StorageError::StorageLock)?;
        let (width, height) = image::image_dimensions(source)?;
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let filename = format!("capture-{timestamp}-{}.png", &id[..8]);
        let relative_path = Path::new("captures").join(filename);
        let destination = self.captures_dir()?.join(
            relative_path
                .file_name()
                .expect("generated capture path always has a filename"),
        );
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
                title, description, note, tags_json, favorite, ocr_text,
                enrichment_status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', '', '', '[]', 0, '', 'pending')",
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
        title: &str,
        description: &str,
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
             SET title_edited = CASE WHEN title <> ?2 THEN 1 ELSE title_edited END,
                 description_edited = CASE
                     WHEN description <> ?3 THEN 1 ELSE description_edited
                 END,
                 title = ?2,
                 description = ?3,
                 note = ?4,
                 tags_json = ?5,
                 favorite = ?6
             WHERE id = ?1",
            params![
                id,
                clean_field(title, 140),
                clean_field(description, 4_000),
                clean_field(note, 10_000),
                tags_json,
                favorite
            ],
        )?;

        if changed == 0 {
            return Err(StorageError::NotFound(id.to_owned()));
        }

        self.get(id)
    }

    pub fn set_enrichment_status(
        &self,
        id: &str,
        status: &str,
    ) -> Result<CaptureRecord, StorageError> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE captures SET enrichment_status = ?2 WHERE id = ?1",
            params![id, status],
        )?;

        if changed == 0 {
            return Err(StorageError::NotFound(id.to_owned()));
        }

        self.get(id)
    }

    pub fn save_enrichment(
        &self,
        id: &str,
        title: &str,
        description: &str,
        ocr_text: &str,
        status: &str,
    ) -> Result<CaptureRecord, StorageError> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE captures
             SET title = CASE
                     WHEN title_edited = 0 AND TRIM(?2) <> '' THEN ?2 ELSE title
                 END,
                 description = CASE
                     WHEN description_edited = 0 AND TRIM(?3) <> '' THEN ?3 ELSE description
                 END,
                 ocr_text = ?4,
                 enrichment_status = ?5
             WHERE id = ?1",
            params![id, title, description, ocr_text, status],
        )?;

        if changed == 0 {
            return Err(StorageError::NotFound(id.to_owned()));
        }

        self.get(id)
    }

    pub fn delete(&self, id: &str) -> Result<(), StorageError> {
        let _operation = self
            .operation_lock
            .lock()
            .map_err(|_| StorageError::StorageLock)?;
        let capture = self.get(id)?;
        let original = PathBuf::from(&capture.file_path);
        let staged = self.captures_dir()?.join(format!(".{id}.deleting"));

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
                        capture_mode, title, description, note, tags_json, favorite,
                        ocr_text, enrichment_status
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
        let tags_json: String = row.get(9)?;
        let tags = serde_json::from_str(&tags_json).unwrap_or_default();
        let filename = Path::new(&relative_path)
            .file_name()
            .unwrap_or_else(|| Path::new(&relative_path).as_os_str());
        let captures_dir = self
            .captures_dir
            .read()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        Ok(CaptureRecord {
            id: row.get(0)?,
            created_at: row.get(1)?,
            file_path: captures_dir.join(filename).to_string_lossy().into_owned(),
            width: row.get(3)?,
            height: row.get(4)?,
            capture_mode: row.get(5)?,
            title: row.get(6)?,
            description: row.get(7)?,
            note: row.get(8)?,
            tags,
            favorite: row.get::<_, i64>(10)? != 0,
            ocr_text: row.get(11)?,
            enrichment_status: row.get(12)?,
        })
    }

    pub fn storage_location(&self) -> Result<PathBuf, StorageError> {
        self.captures_dir()
    }

    pub fn set_storage_location(
        &self,
        requested: PathBuf,
    ) -> Result<Vec<CaptureRecord>, StorageError> {
        let _operation = self
            .operation_lock
            .lock()
            .map_err(|_| StorageError::StorageLock)?;
        if !requested.is_absolute() {
            return Err(StorageError::InvalidStorageLocation);
        }

        fs::create_dir_all(&requested)?;
        let destination_dir = requested.canonicalize()?;
        let current_dir = self.captures_dir()?.canonicalize()?;
        if destination_dir == current_dir {
            return self.list();
        }

        let captures = self.list()?;
        let mut copied = Vec::new();
        for capture in &captures {
            let source = PathBuf::from(&capture.file_path);
            if !source.exists() {
                continue;
            }
            let Some(filename) = source.file_name() else {
                continue;
            };
            let destination = destination_dir.join(filename);
            if destination.exists() {
                Self::remove_copied_files(&copied);
                return Err(StorageError::DestinationConflict(
                    filename.to_string_lossy().into_owned(),
                ));
            }
            let temporary = destination.with_extension("png.capture-vault-part");
            if let Err(error) =
                fs::copy(&source, &temporary).and_then(|_| fs::rename(&temporary, &destination))
            {
                let _ = fs::remove_file(&temporary);
                Self::remove_copied_files(&copied);
                return Err(error.into());
            }
            copied.push(destination);
        }

        let settings = StorageSettings {
            image_directory: Some(destination_dir.clone()),
        };
        if let Err(error) = self.write_settings(&settings) {
            Self::remove_copied_files(&copied);
            return Err(error);
        }

        {
            let mut active_dir = self
                .captures_dir
                .write()
                .map_err(|_| StorageError::StorageLock)?;
            *active_dir = destination_dir;
        }

        for capture in captures {
            let source = PathBuf::from(capture.file_path);
            if source.starts_with(&current_dir) {
                let _ = fs::remove_file(source);
            }
        }
        let _ = fs::remove_dir(current_dir);

        self.list()
    }

    fn captures_dir(&self) -> Result<PathBuf, StorageError> {
        self.captures_dir
            .read()
            .map(|path| path.clone())
            .map_err(|_| StorageError::StorageLock)
    }

    /// Persist storage settings through a sibling temporary file so a failed
    /// write cannot leave a truncated JSON file that prevents the next launch.
    fn write_settings(&self, settings: &StorageSettings) -> Result<(), StorageError> {
        let contents = serde_json::to_vec_pretty(settings)?;
        let temporary = self.settings_path.with_extension("json.part");
        fs::write(&temporary, contents)?;
        if let Err(error) = fs::rename(&temporary, &self.settings_path) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        Ok(())
    }

    fn remove_copied_files(paths: &[PathBuf]) {
        for path in paths {
            let _ = fs::remove_file(path);
        }
    }
}

fn clean_field(value: &str, maximum: usize) -> String {
    value.trim().chars().take(maximum).collect()
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
                "Example title",
                "A concise description",
                "Reference image",
                &[" Design ".into(), "design".into(), "Ubuntu".into()],
                true,
            )
            .expect("update metadata");
        assert_eq!(updated.title, "Example title");
        assert_eq!(updated.description, "A concise description");
        assert_eq!(updated.tags, vec!["Design", "Ubuntu"]);
        assert!(updated.favorite);

        let enriched = store
            .save_enrichment(
                &capture.id,
                "Generated title",
                "Generated description",
                "Recognized words",
                "complete",
            )
            .expect("save enrichment");
        assert_eq!(enriched.title, "Example title");
        assert_eq!(enriched.description, "A concise description");
        assert_eq!(enriched.ocr_text, "Recognized words");
        assert_eq!(enriched.enrichment_status, "complete");

        let fresh_capture = store
            .import_capture(&source, CaptureMode::Screen)
            .expect("import another capture");
        let first_suggestion = store
            .save_enrichment(
                &fresh_capture.id,
                "First suggestion",
                "First generated description",
                "First OCR",
                "complete",
            )
            .expect("save first suggestion");
        assert_eq!(first_suggestion.title, "First suggestion");
        let refreshed = store
            .save_enrichment(
                &fresh_capture.id,
                "Better semantic title",
                "Better semantic description",
                "Updated OCR",
                "complete",
            )
            .expect("refresh generated metadata");
        assert_eq!(refreshed.title, "Better semantic title");
        assert_eq!(refreshed.description, "Better semantic description");
        let partial = store
            .save_enrichment(&fresh_capture.id, "", "", "OCR without vision", "partial")
            .expect("save partial enrichment");
        assert_eq!(partial.title, "Better semantic title");
        assert_eq!(partial.description, "Better semantic description");
        assert_eq!(partial.enrichment_status, "partial");

        store.delete(&capture.id).expect("delete capture");
        store
            .delete(&fresh_capture.id)
            .expect("delete second capture");
        assert!(store.list().expect("list captures").is_empty());
        assert!(!Path::new(&capture.file_path).exists());
    }

    #[test]
    fn changes_storage_location_and_uses_it_after_restart() {
        let directory = tempdir().expect("temporary directory");
        let library = directory.path().join("library");
        let destination = directory.path().join("screenshots");
        let source = directory.path().join("source.png");
        sample_png(&source);

        let store = CaptureStore::new(library.clone()).expect("create store");
        let original = store
            .import_capture(&source, CaptureMode::Area)
            .expect("import capture");
        let moved = store
            .set_storage_location(destination.clone())
            .expect("change storage location");

        assert_eq!(moved.len(), 1);
        assert!(Path::new(&moved[0].file_path).starts_with(&destination));
        assert!(Path::new(&moved[0].file_path).exists());
        assert!(!Path::new(&original.file_path).exists());

        drop(store);
        let reopened = CaptureStore::new(library).expect("reopen store");
        assert_eq!(
            reopened.storage_location().expect("read storage location"),
            destination.canonicalize().expect("canonical destination")
        );
        let new_capture = reopened
            .import_capture(&source, CaptureMode::Screen)
            .expect("import into selected location");
        assert!(Path::new(&new_capture.file_path).starts_with(&destination));
    }

    #[test]
    fn serializes_capture_import_with_storage_migration() {
        let directory = tempdir().expect("temporary directory");
        let library = directory.path().join("library");
        let destination = directory.path().join("screenshots");
        let source = directory.path().join("source.png");
        sample_png(&source);

        let store = CaptureStore::new(library).expect("create store");
        let gate = store.operation_lock.clone();
        let held_gate = gate.lock().expect("hold operation lock");
        let importer = store.clone();
        let migrator = store.clone();
        let import_source = source.clone();
        let import_destination = destination.clone();

        std::thread::scope(|scope| {
            let import =
                scope.spawn(move || importer.import_capture(&import_source, CaptureMode::Screen));
            let migrate = scope.spawn(move || migrator.set_storage_location(import_destination));

            // Both operations begin together after the lock is released. Their
            // order does not matter, but the capture must end in the active
            // directory whichever operation acquires the lock first.
            drop(held_gate);
            import
                .join()
                .expect("join capture import")
                .expect("import capture");
            migrate
                .join()
                .expect("join storage migration")
                .expect("migrate capture store");
        });

        let captures = store.list().expect("list migrated captures");
        assert_eq!(captures.len(), 1);
        assert!(Path::new(&captures[0].file_path).starts_with(&destination));
        assert!(Path::new(&captures[0].file_path).exists());
    }

    #[test]
    fn migrates_a_version_one_library_without_losing_metadata() {
        let directory = tempdir().expect("temporary directory");
        let library = directory.path().join("library");
        fs::create_dir_all(library.join("captures")).expect("create library directories");
        let connection = Connection::open(library.join("capture-vault.sqlite3"))
            .expect("create version one database");
        connection
            .execute_batch(
                "CREATE TABLE captures (
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
                INSERT INTO captures VALUES (
                    'legacy', '2026-09-20T12:00:00.000Z', 'captures/legacy.png',
                    100, 80, 'area', 'Keep me', '[\"legacy\"]', 1
                );
                PRAGMA user_version = 1;",
            )
            .expect("write version one schema");
        drop(connection);

        let store = CaptureStore::new(library).expect("migrate store");
        let capture = store.get("legacy").expect("load migrated capture");

        assert_eq!(capture.note, "Keep me");
        assert_eq!(capture.tags, vec!["legacy"]);
        assert!(capture.favorite);
        assert!(capture.title.is_empty());
        assert!(capture.ocr_text.is_empty());
        assert_eq!(capture.enrichment_status, "pending");

        let connection = store.connection().expect("open migrated database");
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read migrated version");
        assert_eq!(version, 3);
    }

    #[test]
    fn resumes_a_partially_applied_version_one_migration() {
        let directory = tempdir().expect("temporary directory");
        let library = directory.path().join("library");
        fs::create_dir_all(library.join("captures")).expect("create library directories");
        let connection = Connection::open(library.join("capture-vault.sqlite3"))
            .expect("create partial version one database");
        connection
            .execute_batch(
                "CREATE TABLE captures (
                    id TEXT PRIMARY KEY NOT NULL,
                    created_at TEXT NOT NULL,
                    relative_path TEXT NOT NULL UNIQUE,
                    width INTEGER NOT NULL,
                    height INTEGER NOT NULL,
                    capture_mode TEXT NOT NULL,
                    note TEXT NOT NULL DEFAULT '',
                    tags_json TEXT NOT NULL DEFAULT '[]',
                    favorite INTEGER NOT NULL DEFAULT 0,
                    title TEXT NOT NULL DEFAULT '',
                    description TEXT NOT NULL DEFAULT ''
                );
                PRAGMA user_version = 1;",
            )
            .expect("write partially migrated schema");
        drop(connection);

        let store = CaptureStore::new(library).expect("resume migration");
        let connection = store.connection().expect("open migrated database");
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read migrated version");
        let column_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('captures')",
                [],
                |row| row.get(0),
            )
            .expect("count migrated columns");

        assert_eq!(version, 3);
        assert_eq!(column_count, 15);
    }
}
