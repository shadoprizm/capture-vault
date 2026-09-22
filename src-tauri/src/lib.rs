mod capture;
mod enrichment;
mod models;
mod storage;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clipboard_rs::{
    Clipboard, ClipboardContent, ClipboardContext, RustImageData, common::RustImage,
};
use enrichment::EnrichmentEngine;
use models::{CaptureMode, CaptureRecord};
use storage::CaptureStore;
use tauri::{AppHandle, Emitter, Manager, State, path::BaseDirectory};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageLocationUpdate {
    path: String,
    captures: Vec<CaptureRecord>,
}

struct AppState {
    store: CaptureStore,
    enrichment: Option<Arc<EnrichmentEngine>>,
    enrichment_error: Option<String>,
}

#[tauri::command]
fn list_captures(state: State<'_, AppState>) -> Result<Vec<CaptureRecord>, String> {
    state.store.list().map_err(|error| error.to_string())
}

#[tauri::command]
fn get_storage_location(state: State<'_, AppState>) -> Result<String, String> {
    state
        .store
        .storage_location()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_storage_location(
    path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<StorageLocationUpdate, String> {
    let requested = PathBuf::from(path);
    let preview_path = requested
        .canonicalize()
        .map_err(|error| format!("Could not access that folder: {error}"))?;
    app.asset_protocol_scope()
        .allow_directory(preview_path, false)
        .map_err(|error| format!("Could not allow previews from that folder: {error}"))?;
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let captures = store
            .set_storage_location(requested)
            .map_err(|error| error.to_string())?;
        let path = store
            .storage_location()
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .into_owned();
        Ok(StorageLocationUpdate { path, captures })
    })
    .await
    .map_err(|error| format!("Storage migration stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn capture_screen(
    mode: CaptureMode,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CaptureRecord, String> {
    let source = capture::take_screenshot(mode)
        .await
        .map_err(|error| error.to_string())?;

    let created = state
        .store
        .import_capture(source.path(), mode)
        .map_err(|error| error.to_string())?;

    if let Some(engine) = state.enrichment.clone() {
        let processing = state
            .store
            .set_enrichment_status(&created.id, "processing")
            .map_err(|error| error.to_string())?;
        let store = state.store.clone();
        let id = created.id;

        tauri::async_runtime::spawn(async move {
            match analyze_capture(store.clone(), engine, id.clone()).await {
                Ok(updated) => {
                    let _ = app.emit("capture-enriched", updated);
                }
                Err(error) => {
                    if let Ok(failed) = store.set_enrichment_status(&id, "failed") {
                        let _ = app.emit("capture-enriched", failed);
                    }
                    let _ = app.emit("capture-enrichment-failed", error);
                }
            }
        });
        Ok(processing)
    } else {
        state
            .store
            .set_enrichment_status(&created.id, "failed")
            .map_err(|error| error.to_string())
    }
}

#[tauri::command]
fn update_capture_metadata(
    id: String,
    title: String,
    description: String,
    note: String,
    tags: Vec<String>,
    favorite: bool,
    state: State<'_, AppState>,
) -> Result<CaptureRecord, String> {
    state
        .store
        .update_metadata(&id, &title, &description, &note, &tags, favorite)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn enrich_capture(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CaptureRecord, String> {
    let engine = state.enrichment.clone().ok_or_else(|| {
        state
            .enrichment_error
            .clone()
            .unwrap_or_else(|| "Local screenshot analysis is unavailable".into())
    })?;
    let store = state.store.clone();
    match analyze_capture(store.clone(), engine, id.clone()).await {
        Ok(updated) => {
            let _ = app.emit("capture-enriched", updated.clone());
            Ok(updated)
        }
        Err(error) => {
            if let Ok(failed) = store.set_enrichment_status(&id, "failed") {
                let _ = app.emit("capture-enriched", failed);
            }
            Err(error)
        }
    }
}

async fn analyze_capture(
    store: CaptureStore,
    engine: Arc<EnrichmentEngine>,
    id: String,
) -> Result<CaptureRecord, String> {
    let capture = store
        .set_enrichment_status(&id, "processing")
        .map_err(|error| error.to_string())?;
    let image_path = capture.file_path;
    let result =
        tauri::async_runtime::spawn_blocking(move || engine.analyze(Path::new(&image_path)))
            .await
            .map_err(|error| format!("Local analysis task stopped unexpectedly: {error}"))??;

    store
        .save_enrichment(
            &id,
            &result.title,
            &result.description,
            &result.ocr_text,
            result.status,
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_capture(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.store.delete(&id).map_err(|error| error.to_string())
}

#[tauri::command]
fn copy_capture(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let capture = state.store.get(&id).map_err(|error| error.to_string())?;
    let image = RustImageData::from_path(&capture.file_path)
        .map_err(|error| format!("Could not read the screenshot for copying: {error}"))?;
    let clipboard = ClipboardContext::new()
        .map_err(|error| format!("Could not access the system clipboard: {error}"))?;

    let mut contents = vec![
        ClipboardContent::Files(vec![capture.file_path.clone()]),
        ClipboardContent::Image(image),
    ];

    // Linux file managers use this MIME type to distinguish copying from moving.
    // `Files` also publishes the standard text/uri-list representation.
    #[cfg(target_os = "linux")]
    if let Ok(uri) = url::Url::from_file_path(&capture.file_path) {
        contents.push(ClipboardContent::Other(
            "x-special/gnome-copied-files".into(),
            format!("copy\n{uri}").into_bytes(),
        ));
    }

    clipboard
        .set(contents)
        .map_err(|error| format!("Could not copy the screenshot: {error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let store = CaptureStore::new(data_dir)?;
            app.asset_protocol_scope()
                .allow_directory(store.storage_location()?, false)?;
            let detection_model = app.path().resolve(
                "resources/ocr/text-detection-ssfbcj81.rten",
                BaseDirectory::Resource,
            )?;
            let recognition_model = app.path().resolve(
                "resources/ocr/text-rec-checkpoint-s52qdbqt.rten",
                BaseDirectory::Resource,
            )?;
            let (enrichment, enrichment_error) =
                match EnrichmentEngine::load(&detection_model, &recognition_model) {
                    Ok(engine) => (Some(Arc::new(engine)), None),
                    Err(error) => {
                        eprintln!("CaptureVault local analysis is unavailable: {error}");
                        (None, Some(error))
                    }
                };
            app.manage(AppState {
                store,
                enrichment,
                enrichment_error,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_captures,
            get_storage_location,
            set_storage_location,
            capture_screen,
            update_capture_metadata,
            enrich_capture,
            delete_capture,
            copy_capture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
