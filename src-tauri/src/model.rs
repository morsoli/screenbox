use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn validate(&self) -> Result<(), String> {
        if [self.x, self.y, self.width, self.height]
            .iter()
            .any(|v| !v.is_finite())
            || self.x < 0.0
            || self.y < 0.0
            || self.width <= 0.0
            || self.height <= 0.0
            || self.x + self.width > 1.0 + 1e-8
            || self.y + self.height > 1.0 + 1e-8
        {
            return Err("Crop or region must fit inside one source".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Callout {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditDocument {
    pub trim_start_ms: u64,
    pub trim_end_ms: u64,
    pub crop: Rect,
    pub auto_zoom: bool,
    pub zoom_strength: f64,
    pub padding: f64,
    pub cursor_highlight: bool,
    pub callouts: Vec<Callout>,
    pub preset: String,
}

impl EditDocument {
    pub fn default_for(duration_ms: u64) -> Self {
        Self {
            trim_start_ms: 0,
            trim_end_ms: duration_ms,
            crop: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            auto_zoom: true,
            zoom_strength: 1.35,
            padding: 0.06,
            cursor_highlight: true,
            callouts: vec![],
            preset: "h264".into(),
        }
    }
    pub fn validate(&self, duration_ms: u64) -> Result<(), String> {
        if self.trim_start_ms >= self.trim_end_ms || self.trim_end_ms > duration_ms {
            return Err("Trim must be within the recording".into());
        }
        self.crop.validate()?;
        if !self.zoom_strength.is_finite() || !(1.0..=2.5).contains(&self.zoom_strength) {
            return Err("Zoom must be between 1 and 2.5".into());
        }
        if !self.padding.is_finite() || !(0.0..=0.2).contains(&self.padding) {
            return Err("Padding must be between 0 and 20%".into());
        }
        if self.preset != "h264" && self.preset != "webm" {
            return Err("Unsupported export preset".into());
        }
        if self.callouts.len() > 100 {
            return Err("Too many callouts".into());
        }
        for callout in &self.callouts {
            if callout.start_ms >= callout.end_ms
                || callout.end_ms > duration_ms
                || callout.text.trim().is_empty()
                || callout.text.chars().count() > 200
                || !callout.x.is_finite()
                || !callout.y.is_finite()
                || !(0.0..=1.0).contains(&callout.x)
                || !(0.0..=1.0).contains(&callout.y)
            {
                return Err("Callout timing, text, or position is invalid".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub title: String,
    pub original_path: String,
    pub cursor_path: String,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub microphone: bool,
    pub created_at: String,
    pub edit: EditDocument,
    pub last_export_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPoint {
    pub t_ms: u64,
    pub x: f64,
    pub y: f64,
}

pub fn safe_filename(input: &str) -> String {
    let result: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    let trimmed = result.trim_matches(['-', '_']);
    if trimmed.is_empty() {
        "recording".into()
    } else {
        trimmed.into()
    }
}

pub fn media_tool(name: &str) -> String {
    let variable = if name == "ffprobe" {
        "SCREENBOX_FFPROBE"
    } else {
        "SCREENBOX_FFMPEG"
    };
    if let Ok(path) = std::env::var(variable) {
        if !path.trim().is_empty() {
            return path;
        }
    }
    for prefix in ["/opt/homebrew/bin", "/usr/local/bin"] {
        let path = std::path::Path::new(prefix).join(name);
        if path.is_file() {
            return path.to_string_lossy().to_string();
        }
    }
    name.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_trim() {
        let mut e = EditDocument::default_for(1000);
        e.trim_start_ms = 1000;
        assert!(e.validate(1000).is_err());
    }
    #[test]
    fn rejects_outside_crop() {
        assert!(
            Rect {
                x: 0.8,
                y: 0.0,
                width: 0.4,
                height: 1.0
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn sanitizes_filename() {
        assert_eq!(safe_filename("../my demo: one"), "my-demo_-one");
    }
}
