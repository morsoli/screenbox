use crate::{
    data::Database,
    model::{EditDocument, Project, Rect},
};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::Mutex,
    time::Instant,
};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureConfig {
    pub source_id: String,
    pub kind: String,
    pub region: Option<Rect>,
    pub microphone: bool,
}

pub struct ActiveRecording {
    child: Child,
    stdin: ChildStdin,
    started: Instant,
    id: String,
    title: String,
    original_path: String,
    cursor_path: String,
    width: u32,
    height: u32,
    microphone: bool,
}
#[derive(Default)]
pub struct RecordingState {
    active: Option<ActiveRecording>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub recording: bool,
    pub elapsed_ms: u64,
}

fn helper_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("resources/screenbox-capture");
    if bundled.exists() {
        return Ok(bundled);
    }
    let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/screenbox-capture");
    if dev.exists() {
        Ok(dev)
    } else {
        Err("Capture helper is missing. Run ./dev.sh again.".into())
    }
}

#[tauri::command]
pub fn list_sources(app: AppHandle) -> Result<Vec<CaptureSource>, String> {
    let output = Command::new(helper_path(&app)?)
        .arg("sources")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let message = if output.stderr.is_empty() {
            &output.stdout
        } else {
            &output.stderr
        };
        let description = String::from_utf8_lossy(message).trim().to_string();
        if description.contains("TCC") || description.to_lowercase().contains("permission") {
            return Err("Screen Recording permission is unavailable. Allow Screenbox in System Settings → Privacy & Security, then reopen the app.".into());
        }
        return Err(description);
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_recording(
    app: AppHandle,
    db: State<'_, Database>,
    state: State<'_, Mutex<RecordingState>>,
    config: CaptureConfig,
) -> Result<(), String> {
    let sources = list_sources(app.clone())?;
    let source = sources
        .into_iter()
        .find(|s| s.id == config.source_id && s.kind == config.kind)
        .ok_or("Selected source is no longer available")?;
    if let Some(region) = &config.region {
        region.validate()?;
    }
    if config.kind == "region" && config.region.is_none() {
        return Err("Choose a region before recording".into());
    }
    if config.kind != "region" && config.region.is_some() {
        return Err("Region coordinates belong to region capture only".into());
    }
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    if guard.active.is_some() {
        return Err("A recording is already in progress".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let directory = db.root.join("recordings").join(&id);
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let original_path = directory.join("original.mp4").to_string_lossy().to_string();
    let cursor_path = directory.join("cursor.jsonl").to_string_lossy().to_string();
    let mut child = Command::new(helper_path(&app)?)
        .arg("record")
        .arg(serde_json::to_string(&config).map_err(|e| e.to_string())?)
        .arg(&original_path)
        .arg(&cursor_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Capture helper output unavailable")?;
    let mut first = String::new();
    BufReader::new(stdout)
        .read_line(&mut first)
        .map_err(|e| e.to_string())?;
    if first.trim() != "READY" {
        let _ = child.kill();
        return Err(if first.is_empty() {
            "Capture failed. Check Screen Recording and Microphone permissions, then retry.".into()
        } else {
            first.trim().to_string()
        });
    }
    let stdin = child
        .stdin
        .take()
        .ok_or("Capture helper input unavailable")?;
    let (width, height) = config
        .region
        .as_ref()
        .map(|r| {
            (
                (source.width as f64 * r.width) as u32 / 2 * 2,
                (source.height as f64 * r.height) as u32 / 2 * 2,
            )
        })
        .unwrap_or((source.width, source.height));
    guard.active = Some(ActiveRecording {
        child,
        stdin,
        started: Instant::now(),
        id,
        title: source.title,
        original_path,
        cursor_path,
        width,
        height,
        microphone: config.microphone,
    });
    if app.get_webview_window("timer").is_none() {
        let _ = WebviewWindowBuilder::new(
            &app,
            "timer",
            WebviewUrl::App("index.html?overlay=1".into()),
        )
        .title("Recording timer")
        .inner_size(182.0, 52.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .position(24.0, 24.0)
        .skip_taskbar(true)
        .build();
    }
    let _ = app.emit("recording-started", ());
    Ok(())
}

pub fn stop_recording_internal(app: &AppHandle) -> Result<Project, String> {
    let state = app.state::<Mutex<RecordingState>>();
    let mut active = state
        .lock()
        .map_err(|e| e.to_string())?
        .active
        .take()
        .ok_or("No recording in progress")?;
    active
        .stdin
        .write_all(b"stop\n")
        .map_err(|e| e.to_string())?;
    let status = active.child.wait().map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.close();
    }
    if let Some(tray) = app.tray_by_id("screenbox") {
        let _ = tray.set_title(Some("Screenbox"));
    }
    if !status.success() {
        return Err("Recording stopped unexpectedly. The original file remains in Application Support for recovery.".into());
    }
    let ffprobe = crate::model::media_tool("ffprobe");
    let duration_seconds = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &active.original_path,
        ])
        .output()
        .ok()
        .and_then(|output| {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<f64>()
                .ok()
        })
        .unwrap_or(active.started.elapsed().as_secs_f64());
    let duration_ms = (duration_seconds.max(0.1) * 1000.0) as u64;
    let poster_path = std::path::Path::new(&active.original_path).with_extension("poster.jpg");
    let _ = Command::new(crate::model::media_tool("ffmpeg"))
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-ss",
            "0.1",
            "-i",
            &active.original_path,
            "-frames:v",
            "1",
            "-q:v",
            "3",
        ])
        .arg(&poster_path)
        .status();
    let project = Project {
        id: active.id,
        title: active.title,
        original_path: active.original_path,
        cursor_path: active.cursor_path,
        duration_ms,
        width: active.width,
        height: active.height,
        microphone: active.microphone,
        created_at: chrono::Utc::now().to_rfc3339(),
        edit: EditDocument::default_for(duration_ms),
        last_export_path: None,
    };
    app.state::<Database>().insert(&project)?;
    let _ = app.emit("recording-stopped", &project);
    Ok(project)
}

#[tauri::command]
pub fn stop_recording(app: AppHandle) -> Result<Project, String> {
    stop_recording_internal(&app)
}

#[tauri::command]
pub fn recording_status(app: AppHandle) -> Result<RecordingStatus, String> {
    let state = app.state::<Mutex<RecordingState>>();
    let guard = state.lock().map_err(|e| e.to_string())?;
    let elapsed_ms = guard
        .active
        .as_ref()
        .map(|a| a.started.elapsed().as_millis() as u64)
        .unwrap_or(0);
    if let Some(tray) = app.tray_by_id("screenbox") {
        let title = if guard.active.is_some() {
            format!("● {:02}:{:02}", elapsed_ms / 60000, elapsed_ms / 1000 % 60)
        } else {
            "Screenbox".into()
        };
        let _ = tray.set_title(Some(&title));
    }
    Ok(RecordingStatus {
        recording: guard.active.is_some(),
        elapsed_ms,
    })
}

#[tauri::command]
pub fn create_fixture_project(db: State<'_, Database>) -> Result<Project, String> {
    if !cfg!(debug_assertions) {
        return Err("Fixture creation is disabled in release builds".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let directory = db.root.join("recordings").join(&id);
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let original = directory.join("original.mp4");
    let cursor = directory.join("cursor.jsonl");
    let status = Command::new(crate::model::media_tool("ffmpeg"))
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=30",
            "-t",
            "3",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&original)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Could not create synthetic video fixture".into());
    }
    let _ = Command::new(crate::model::media_tool("ffmpeg"))
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-ss",
            "0.1",
            "-i",
        ])
        .arg(&original)
        .args(["-frames:v", "1", "-q:v", "3"])
        .arg(directory.join("original.poster.jpg"))
        .status();
    let points = (0..90)
        .map(|frame| {
            format!(
                "{{\"tMs\":{},\"x\":{:.4},\"y\":{:.4}}}",
                frame * 33,
                0.2 + 0.6 * frame as f64 / 89.0,
                0.5
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&cursor, format!("{points}\n")).map_err(|e| e.to_string())?;
    let project = Project {
        id,
        title: "E2E sample".into(),
        original_path: original.to_string_lossy().to_string(),
        cursor_path: cursor.to_string_lossy().to_string(),
        duration_ms: 3000,
        width: 640,
        height: 360,
        microphone: false,
        created_at: chrono::Utc::now().to_rfc3339(),
        edit: EditDocument::default_for(3000),
        last_export_path: None,
    };
    db.insert(&project)?;
    Ok(project)
}
