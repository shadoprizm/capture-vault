//! Optional, explicit hosted backup. Device keys remain in memory and are never
//! written to settings. All requests are pinned to our HTTPS origin; redirects
//! are disabled so credentials cannot be forwarded to another service.
use std::{fs, io::Read, time::Duration};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    models::{CaptureMode, CaptureRecord},
    storage::CaptureStore,
};

const ORIGIN: &str = "https://capturerecall.com";
const MAX_PNG: u64 = 10 * 1024 * 1024;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudConnection {
    pub workspace: String,
    pub token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudCapture {
    pub id: String,
    pub sha256: String,
    pub bytes: u64,
    pub metadata: CloudMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudMetadata {
    pub title: String,
    pub description: String,
    pub note: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub ocr_text: String,
    pub capture_mode: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudPage {
    pub captures: Vec<CloudCapture>,
    pub next_cursor: Option<String>,
}

fn client(connection: &CloudConnection) -> Result<reqwest::blocking::Client, String> {
    uuid::Uuid::parse_str(&connection.workspace)
        .map_err(|_| "Enter the library ID from your hosted account.".to_string())?;
    let key = connection
        .token
        .strip_prefix("cr_")
        .ok_or("Enter a valid device key.")?;
    if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Enter a valid device key.".into());
    }
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| "Could not prepare the secure hosted connection.".into())
}

fn response(mut response: reqwest::blocking::Response, limit: u64) -> Result<Vec<u8>, String> {
    if !response.status().is_success() {
        let status = response.status();
        let mut bytes = Vec::new();
        let _ = response.by_ref().take(4096).read_to_end(&mut bytes);
        let message = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| format!("Hosted vault returned {status}. Please retry."));
        return Err(message);
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not download the hosted response.")?;
    if bytes.len() as u64 > limit {
        return Err("Hosted response exceeds the supported size.".into());
    }
    Ok(bytes)
}

pub fn list(connection: &CloudConnection, cursor: Option<String>) -> Result<CloudPage, String> {
    let mut query = vec![("workspace", connection.workspace.clone())];
    if let Some(cursor) = cursor {
        query.push(("cursor", cursor));
    }
    let response_bytes = response(
        client(connection)?
            .get(format!("{ORIGIN}/api/vault/list"))
            .query(&query)
            .bearer_auth(&connection.token)
            .send()
            .map_err(|_| "Could not reach your hosted vault.")?,
        32 * 1024 * 1024,
    )?;
    serde_json::from_slice::<CloudPage>(&response_bytes)
        .map_err(|_| "Could not read the hosted library.".into())
}

pub fn upload(connection: &CloudConnection, store: &CaptureStore, id: &str) -> Result<(), String> {
    let client = client(connection)?;
    let capture = store.get(id).map_err(|e| e.to_string())?;
    let file = fs::File::open(&capture.file_path).map_err(|_| "Could not read this screenshot.")?;
    let mut bytes = Vec::new();
    file.take(MAX_PNG + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read this screenshot.")?;
    if bytes.len() as u64 > MAX_PNG {
        return Err("Hosted screenshots must be 10 MB or smaller.".into());
    }
    // No local file path is included in the upload.
    let metadata = CloudMetadata {
        title: capture.title,
        description: capture.description,
        note: capture.note,
        tags: capture.tags,
        favorite: capture.favorite,
        ocr_text: capture.ocr_text,
        capture_mode: capture.capture_mode,
        created_at: capture.created_at,
    };
    response(client.post(format!("{ORIGIN}/api/vault/upload"))
        .bearer_auth(&connection.token)
        .json(&serde_json::json!({"workspace":connection.workspace,"metadata":metadata,"png":STANDARD.encode(bytes)}))
        .send().map_err(|_| "Could not back up the screenshot. Check your connection and retry.")?, 4096)?;
    Ok(())
}

pub fn restore(
    connection: &CloudConnection,
    store: &CaptureStore,
    id: &str,
) -> Result<CaptureRecord, String> {
    let metadata_bytes = response(
        client(connection)?
            .get(format!("{ORIGIN}/api/vault/capture"))
            .query(&[
                ("workspace", &connection.workspace),
                ("id", &id.to_string()),
            ])
            .bearer_auth(&connection.token)
            .send()
            .map_err(|_| "Could not read the hosted screenshot.")?,
        1024 * 1024,
    )?;
    let capture: CloudCapture = serde_json::from_slice(&metadata_bytes)
        .map_err(|_| "Could not read screenshot metadata.")?;
    uuid::Uuid::parse_str(&capture.id).map_err(|_| "Invalid hosted screenshot ID.")?;
    let bytes = response(
        client(connection)?
            .get(format!("{ORIGIN}/api/vault/download"))
            .query(&[("workspace", &connection.workspace), ("id", &capture.id)])
            .bearer_auth(&connection.token)
            .send()
            .map_err(|_| "Could not download the screenshot.")?,
        MAX_PNG,
    )?;
    verify_download(&bytes, &capture)?;
    import_download(store, &bytes, &capture.metadata)
}

fn import_download(
    store: &CaptureStore,
    bytes: &[u8],
    metadata: &CloudMetadata,
) -> Result<CaptureRecord, String> {
    // Image import identifies the format from the path. Keep the PNG extension
    // on the verified download so the decoder can recognize the temporary file.
    let mut temporary = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .map_err(|_| "Could not prepare the restored screenshot.")?;
    std::io::Write::write_all(&mut temporary, &bytes)
        .map_err(|_| "Could not save the restored screenshot.")?;
    let mode = if metadata.capture_mode == "area" {
        CaptureMode::Area
    } else {
        CaptureMode::Screen
    };
    let record = store
        .import_capture(temporary.path(), mode)
        .map_err(|e| e.to_string())?;
    let result = store.restore_metadata(&record.id, metadata);
    if result.is_err() {
        let _ = store.delete(&record.id);
    }
    result.map_err(|e| e.to_string())
}

fn verify_download(bytes: &[u8], capture: &CloudCapture) -> Result<(), String> {
    if bytes.len() as u64 != capture.bytes
        || format!("{:x}", Sha256::digest(bytes)) != capture.sha256
    {
        return Err("Screenshot integrity check failed. No local capture was changed.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_device_credentials_before_network_access() {
        assert!(
            client(&CloudConnection {
                workspace: "not-a-uuid".into(),
                token: "cr_bad".into()
            })
            .is_err()
        );
        assert!(
            client(&CloudConnection {
                workspace: uuid::Uuid::new_v4().to_string(),
                token: "bad".into()
            })
            .is_err()
        );
    }
    #[test]
    fn verified_png_download_imports_with_metadata_as_a_new_capture() {
        let root = tempfile::tempdir().unwrap();
        let store = CaptureStore::new(root.path().to_path_buf()).unwrap();
        let image = image::RgbImage::from_pixel(8, 6, image::Rgb([24, 96, 48]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let metadata = CloudMetadata {
            title: "Restore regression".into(),
            description: "Verified PNG download".into(),
            note: "Keep this note".into(),
            tags: vec!["round-trip".into()],
            favorite: true,
            ocr_text: "Retained OCR".into(),
            capture_mode: "area".into(),
            created_at: "2026-10-12T12:00:00.000Z".into(),
        };
        let first = import_download(&store, png.get_ref(), &metadata).unwrap();
        let second = import_download(&store, png.get_ref(), &metadata).unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!((first.width, first.height), (8, 6));
        assert_eq!(std::fs::read(&first.file_path).unwrap(), *png.get_ref());
        assert_eq!(first.title, metadata.title);
        assert_eq!(first.description, metadata.description);
        assert_eq!(first.note, metadata.note);
        assert_eq!(first.tags, metadata.tags);
        assert_eq!(first.favorite, metadata.favorite);
        assert_eq!(first.ocr_text, metadata.ocr_text);
        assert_eq!(first.capture_mode, metadata.capture_mode);
        assert_eq!(first.created_at, metadata.created_at);
    }
    #[test]
    fn corrupted_download_is_rejected() {
        let bytes = b"original PNG bytes";
        let capture = CloudCapture {
            id: uuid::Uuid::new_v4().to_string(),
            bytes: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            metadata: CloudMetadata {
                title: "Test".into(),
                description: String::new(),
                note: String::new(),
                tags: vec![],
                favorite: false,
                ocr_text: String::new(),
                capture_mode: "screen".into(),
                created_at: String::new(),
            },
        };
        assert!(verify_download(bytes, &capture).is_ok());
        assert!(verify_download(b"corrupted PNG data", &capture).is_err());
    }
}
