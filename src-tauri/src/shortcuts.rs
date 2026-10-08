use std::{collections::HashSet, sync::Mutex};

use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::{models::CaptureMode, start_shortcut_capture};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    enabled: bool,
    modifiers: String,
    key: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShortcutSettings {
    area: ShortcutBinding,
    screen: ShortcutBinding,
}

impl ShortcutSettings {
    fn entries(&self) -> Result<Vec<(CaptureMode, String)>, String> {
        let mut entries = Vec::new();
        let mut unique = HashSet::new();
        for (mode, binding) in [
            (CaptureMode::Area, &self.area),
            (CaptureMode::Screen, &self.screen),
        ] {
            if !binding.enabled {
                continue;
            }
            let valid_modifiers = matches!(
                binding.modifiers.as_str(),
                "Control+Alt"
                    | "Control+Shift"
                    | "Alt+Shift"
                    | "Super+Shift"
                    | "Super+Alt"
                    | "Control+Super"
                    | "Control+Alt+Shift"
            );
            let valid_key = binding.key.len() == 1
                && binding.key.as_bytes()[0].is_ascii_alphanumeric()
                || (1..=12).any(|number| binding.key == format!("F{number}"));
            if !valid_modifiers || !valid_key {
                return Err("Choose a supported shortcut combination".into());
            }
            let accelerator = format!("{}+{}", binding.modifiers, binding.key);
            if !unique.insert(accelerator.clone()) {
                return Err("Area capture and full screen cannot use the same shortcut".into());
            }
            entries.push((mode, accelerator));
        }
        Ok(entries)
    }
}

#[derive(Default)]
pub struct ShortcutRegistry(Mutex<Option<ShortcutSettings>>);

impl ShortcutRegistry {
    pub fn configure(&self, app: &AppHandle, settings: ShortcutSettings) -> Result<(), String> {
        let entries = settings.entries()?;
        let mut current = self.0.lock().map_err(|_| "Could not update shortcuts")?;
        let previous = current.clone();
        app.global_shortcut()
            .unregister_all()
            .map_err(|error| error.to_string())?;

        for (mode, accelerator) in entries {
            let registration =
                app.global_shortcut()
                    .on_shortcut(accelerator.as_str(), move |app, _, event| {
                        if event.state == ShortcutState::Pressed {
                            start_shortcut_capture(app, mode);
                        }
                    });
            if let Err(error) = registration {
                let _ = app.global_shortcut().unregister_all();
                if let Some(previous) = previous.as_ref() {
                    for (old_mode, old_accelerator) in previous.entries()? {
                        let _ = app.global_shortcut().on_shortcut(
                            old_accelerator.as_str(),
                            move |app, _, event| {
                                if event.state == ShortcutState::Pressed {
                                    start_shortcut_capture(app, old_mode);
                                }
                            },
                        );
                    }
                }
                return Err(format!("Could not register {accelerator}: {error}"));
            }
        }
        *current = Some(settings);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_or_invalid_shortcuts_before_changing_registration() {
        let area = ShortcutBinding {
            enabled: true,
            modifiers: "Alt+Shift".into(),
            key: "S".into(),
        };
        let settings = ShortcutSettings {
            area: area.clone(),
            screen: area.clone(),
        };
        assert!(settings.entries().is_err());

        let settings = ShortcutSettings {
            area,
            screen: ShortcutBinding {
                enabled: true,
                modifiers: "Super+Shift".into(),
                key: "Escape".into(),
            },
        };
        assert!(settings.entries().is_err());
    }
}
