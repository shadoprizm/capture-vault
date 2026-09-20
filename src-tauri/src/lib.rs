mod capture;
mod models;
mod storage;

use clipboard_rs::{
    Clipboard, ClipboardContent, ClipboardContext, RustImageData, common::RustImage,
};
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
            delete_capture,
            copy_capture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
