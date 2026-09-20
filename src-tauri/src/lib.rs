mod capture;
mod models;
mod storage;

use models::{CaptureMode, CaptureRecord};
use storage::CaptureStore;
use tauri::{Manager, State};

struct AppState {
    store: CaptureStore,
}

#[tauri::command]
fn list_captures(state: State<'_, AppState>) -> Result<Vec<CaptureRecord>, String> {
    state.store.list().map_err(|error| error.to_string())
}

#[tauri::command]
async fn capture_screen(
    mode: CaptureMode,
    state: State<'_, AppState>,
) -> Result<CaptureRecord, String> {
    let source = capture::take_screenshot(mode)
        .await
        .map_err(|error| error.to_string())?;

    state
        .store
        .import_capture(&source, mode)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_capture_metadata(
    id: String,
    note: String,
    tags: Vec<String>,
    favorite: bool,
    state: State<'_, AppState>,
) -> Result<CaptureRecord, String> {
    state
        .store
        .update_metadata(&id, &note, &tags, favorite)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_capture(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.store.delete(&id).map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let store = CaptureStore::new(data_dir)?;
            app.manage(AppState { store });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_captures,
            capture_screen,
            update_capture_metadata,
            delete_capture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
