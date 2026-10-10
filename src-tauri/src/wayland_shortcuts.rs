//! Wayland shortcuts must be registered with the compositor, not XWayland.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

#[derive(Clone, Deserialize)]
pub struct ShortcutBinding {
    enabled: bool,
    modifiers: String,
    key: String,
}

#[derive(Clone, Deserialize)]
pub struct ShortcutSettings {
    area: ShortcutBinding,
    screen: ShortcutBinding,
}

#[derive(Clone, Default, Serialize)]
pub struct ShortcutLabels {
    area: Option<String>,
    screen: Option<String>,
}

#[tauri::command]
pub fn uses_wayland_shortcuts() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
            || std::env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty()))
}

#[tauri::command]
pub async fn configure_wayland_shortcuts(
    settings: ShortcutSettings,
    app: AppHandle,
    state: State<'_, ShortcutState>,
) -> Result<ShortcutLabels, String> {
    #[cfg(target_os = "linux")]
    {
        linux::configure(settings, app, &state).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (settings, app, state);
        Err("Wayland shortcuts are only available on Linux.".into())
    }
}

#[tauri::command]
pub async fn unregister_wayland_shortcuts(state: State<'_, ShortcutState>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let mut registration = state.registration.lock().await;
        if let Some(previous) = registration.take() {
            previous.stop().await?;
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = state;
    Ok(())
}

#[cfg(target_os = "linux")]
#[derive(Default)]
pub struct ShortcutState {
    registration: tokio::sync::Mutex<Option<linux::Registration>>,
    identity_error: Option<String>,
}

#[cfg(not(target_os = "linux"))]
#[derive(Default)]
pub struct ShortcutState;

pub fn initialize(app: &AppHandle) -> ShortcutState {
    #[cfg(target_os = "linux")]
    {
        let identity_error = if uses_wayland_shortcuts() {
            linux::register_identity(app).err()
        } else {
            None
        };
        ShortcutState {
            identity_error,
            ..Default::default()
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        ShortcutState
    }
}

impl ShortcutBinding {
    fn portal_trigger(&self) -> Result<String, String> {
        let modifiers = match self.modifiers.as_str() {
            "Control+Alt" => "CTRL+ALT",
            "Control+Shift" => "CTRL+SHIFT",
            "Alt+Shift" => "ALT+SHIFT",
            "Super+Shift" => "LOGO+SHIFT",
            "Super+Alt" => "LOGO+ALT",
            "Control+Super" => "CTRL+LOGO",
            "Control+Alt+Shift" => "CTRL+ALT+SHIFT",
            _ => return Err("Unsupported shortcut modifiers.".into()),
        };
        let key = if self.key.len() == 1
            && self
                .key
                .bytes()
                .all(|key| key.is_ascii_uppercase() || key.is_ascii_digit())
        {
            // XDG shortcut triggers use the base keysym even with Shift held.
            self.key.to_ascii_lowercase()
        } else if (1..=12).any(|number| self.key == format!("F{number}")) {
            self.key.clone()
        } else {
            return Err("Unsupported shortcut key.".into());
        };
        Ok(format!("{modifiers}+{key}"))
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use ashpd::desktop::{
        Session,
        global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut},
    };
    use futures_util::StreamExt;
    use std::{collections::HashMap, sync::Arc};
    use tauri::Emitter;

    use super::*;

    pub fn register_identity(app: &AppHandle) -> Result<(), String> {
        use tauri::Manager;
        let id = &app.config().identifier;
        // Portable AppImages also need a desktop identity for GNOME to name
        // the app and persist its shortcut permission. NoDisplay avoids adding
        // a duplicate launcher alongside an existing installation.
        let desktop_dir = app
            .path()
            .local_data_dir()
            .map_err(|error| error.to_string())?
            .join("applications");
        std::fs::create_dir_all(&desktop_dir).map_err(|error| error.to_string())?;
        let executable = std::env::var_os("APPIMAGE")
            .map(std::path::PathBuf::from)
            .unwrap_or(std::env::current_exe().map_err(|error| error.to_string())?);
        let executable = executable
            .to_string_lossy()
            .replace('\\', "\\\\\\\\")
            .replace('"', "\\\\\"")
            .replace('`', "\\\\`")
            .replace('$', "\\\\$")
            .replace('%', "%%");
        let identity = format!(
            "[Desktop Entry]\nType=Application\nName=CaptureRecall\nExec=\"{executable}\"\nIcon=capture-vault\nNoDisplay=true\nTerminal=false\n"
        );
        std::fs::write(desktop_dir.join(format!("{id}.desktop")), identity)
            .map_err(|error| format!("Could not install the desktop shortcut identity: {error}"))?;
        let app_id = id
            .parse()
            .map_err(|error| format!("Invalid portal application ID: {error}"))?;
        tauri::async_runtime::block_on(ashpd::register_host_app(app_id)).map_err(|error| {
            format!("Could not register CaptureRecall with the desktop portal: {error}")
        })
    }

    pub struct Registration {
        session: Arc<Session<GlobalShortcuts>>,
        listener: tauri::async_runtime::JoinHandle<()>,
    }

    impl Registration {
        pub async fn stop(self) -> Result<(), String> {
            self.listener.abort();
            self.session
                .close()
                .await
                .map_err(|error| error.to_string())
        }
    }

    fn labels(shortcuts: &[Shortcut], enabled: &HashMap<String, String>) -> ShortcutLabels {
        let mut labels = ShortcutLabels::default();
        for shortcut in shortcuts {
            if !enabled.contains_key(shortcut.id()) {
                continue;
            }
            let label = Some(shortcut.trigger_description().to_owned());
            match shortcut.id() {
                "area" => labels.area = label,
                "screen" => labels.screen = label,
                _ => {}
            }
        }
        labels
    }

    pub async fn configure(
        settings: ShortcutSettings,
        app: AppHandle,
        state: &ShortcutState,
    ) -> Result<ShortcutLabels, String> {
        if !uses_wayland_shortcuts() {
            return Err("This desktop does not use Wayland shortcuts.".into());
        }
        let mut enabled = HashMap::new();
        let mut requested = Vec::new();
        for (id, description, binding) in [
            ("area", "Capture an area with CaptureRecall", settings.area),
            (
                "screen",
                "Capture full screen with CaptureRecall",
                settings.screen,
            ),
        ] {
            if !binding.enabled {
                continue;
            }
            let trigger = binding.portal_trigger()?;
            if enabled.values().any(|existing| existing == &trigger) {
                return Err("Area capture and full screen cannot use the same shortcut.".into());
            }
            requested.push(NewShortcut::new(id, description).preferred_trigger(trigger.as_str()));
            enabled.insert(id.to_owned(), trigger);
        }

        // Keep registration and replacement serialized even if multiple windows
        // or frontend reloads request changes at the same time.
        if let Some(error) = &state.identity_error {
            return Err(error.clone());
        }
        let mut registration = state.registration.lock().await;
        if let Some(previous) = registration.take() {
            previous.stop().await?;
        }
        if requested.is_empty() {
            return Ok(ShortcutLabels::default());
        }

        let portal = GlobalShortcuts::new().await.map_err(|error| format!(
            "This Wayland desktop could not provide global shortcuts: {error}. Enable its Global Shortcuts portal or use an X11 session."
        ))?;
        let session = Arc::new(
            portal
                .create_session(Default::default())
                .await
                .map_err(|error| {
                    format!("Could not create the desktop shortcut session: {error}")
                })?,
        );

        // Subscribe before binding so the first activation cannot be lost.
        let result = async {
            let activated = portal.receive_activated().await?;
            let changed = portal.receive_shortcuts_changed().await?;
            let response = portal
                .bind_shortcuts(session.as_ref(), &requested, None, Default::default())
                .await?
                .response()?;
            Ok::<_, ashpd::Error>((activated, changed, response))
        }
        .await;
        let (activated, changed, response) = match result {
            Ok(result) => result,
            Err(error) => {
                let _ = session.close().await;
                return Err(format!("Desktop shortcut setup was not completed: {error}"));
            }
        };
        let active_labels = labels(response.shortcuts(), &enabled);
        // Session's serialized value is its D-Bus object path. Signals from
        // other applications sharing the portal connection must be ignored.
        let session_path =
            serde_json::to_value(session.as_ref()).map_err(|error| error.to_string())?;
        let session_path = session_path.as_str().unwrap_or_default().to_owned();
        let listener_session = session.clone();
        let listener = tauri::async_runtime::spawn(async move {
            let closed = match listener_session.receive_closed().await {
                Ok(closed) => closed,
                Err(error) => {
                    let _ = app.emit("capture-shortcut-unavailable", error.to_string());
                    return;
                }
            };
            futures_util::pin_mut!(activated, changed, closed);
            loop {
                tokio::select! {
                    event = activated.next() => {
                        let Some(event) = event else { break; };
                        if event.session_handle().as_str() == session_path
                            && enabled.contains_key(event.shortcut_id()) {
                            let mode = match event.shortcut_id() {
                                "area" => crate::models::CaptureMode::Area,
                                "screen" => crate::models::CaptureMode::Screen,
                                _ => continue,
                            };
                            crate::start_shortcut_capture(&app, mode);
                        }
                    }
                    event = changed.next() => {
                        let Some(event) = event else { break; };
                        if event.session_handle().as_str() == session_path {
                            let _ = app.emit("capture-shortcut-labels", labels(event.shortcuts(), &enabled));
                        }
                    }
                    _ = closed.next() => { break; }
                }
            }
            let _ = app.emit(
                "capture-shortcut-unavailable",
                "Desktop global shortcuts disconnected. Reapply your shortcuts in Settings.",
            );
        });
        *registration = Some(Registration { session, listener });
        Ok(active_labels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger(modifiers: &str, key: &str) -> Result<String, String> {
        ShortcutBinding {
            enabled: true,
            modifiers: modifiers.into(),
            key: key.into(),
        }
        .portal_trigger()
    }

    #[test]
    fn translates_modifiers_and_base_keysyms() {
        for (modifiers, expected) in [
            ("Control+Alt", "CTRL+ALT"),
            ("Control+Shift", "CTRL+SHIFT"),
            ("Alt+Shift", "ALT+SHIFT"),
            ("Super+Shift", "LOGO+SHIFT"),
            ("Super+Alt", "LOGO+ALT"),
            ("Control+Super", "CTRL+LOGO"),
            ("Control+Alt+Shift", "CTRL+ALT+SHIFT"),
        ] {
            assert_eq!(trigger(modifiers, "A").unwrap(), format!("{expected}+a"));
        }
        assert_eq!(trigger("Control+Alt", "F12").unwrap(), "CTRL+ALT+F12");
        assert_eq!(trigger("Control+Alt", "0").unwrap(), "CTRL+ALT+0");
    }

    #[test]
    fn rejects_invalid_shortcut_settings() {
        assert!(trigger("Control", "A").is_err());
        for key in ["F0", "F13", "AA", "a", "", "Return", "!"] {
            assert!(trigger("Control+Alt", key).is_err());
        }
    }
}
