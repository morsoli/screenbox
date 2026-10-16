use crate::data::Database;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State, menu::{MenuBuilder, MenuItemBuilder}};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferences {
    pub language: LanguagePreference,
    pub theme: ThemePreference,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum LanguagePreference {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "zh-CN")]
    Chinese,
    #[serde(rename = "en")]
    English,
}

impl LanguagePreference {
    pub fn as_str(&self) -> &'static str {
        match self { Self::System => "system", Self::Chinese => "zh-CN", Self::English => "en" }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ThemePreference {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

impl ThemePreference {
    pub fn as_str(&self) -> &'static str {
        match self { Self::System => "system", Self::Light => "light", Self::Dark => "dark" }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum UiLocale {
    #[serde(rename = "zh-CN")]
    Chinese,
    #[serde(rename = "en")]
    English,
}

#[tauri::command]
pub fn save_preferences(
    app: AppHandle,
    db: State<'_, Database>,
    preferences: UserPreferences,
) -> Result<(), String> {
    db.save_preferences(&preferences)?;
    app.emit("preferences-changed", preferences).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_tray_language(app: AppHandle, locale: UiLocale) -> Result<(), String> {
    let Some(tray) = app.tray_by_id("screenbox") else { return Ok(()) };
    let (show_label, stop_label) = match locale {
        UiLocale::Chinese => ("显示 Screenbox", "停止录制"),
        UiLocale::English => ("Show Screenbox", "Stop recording"),
    };
    let show = MenuItemBuilder::with_id("show", show_label).build(&app).map_err(|e| e.to_string())?;
    let stop = MenuItemBuilder::with_id("stop", stop_label).build(&app).map_err(|e| e.to_string())?;
    let menu = MenuBuilder::new(&app).item(&show).item(&stop).build().map_err(|e| e.to_string())?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_serialize_as_stable_local_values() {
        let preferences = UserPreferences { language: LanguagePreference::Chinese, theme: ThemePreference::Dark };
        assert_eq!(serde_json::to_string(&preferences).unwrap(), r#"{"language":"zh-CN","theme":"dark"}"#);
        assert_eq!(serde_json::from_str::<UserPreferences>(r#"{"language":"system","theme":"light"}"#).unwrap(), UserPreferences { language: LanguagePreference::System, theme: ThemePreference::Light });
    }
}
