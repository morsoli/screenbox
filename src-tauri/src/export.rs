use crate::{
    data::Database,
    model::{Callout, CursorPoint, Project, safe_filename},
    settings::UiLocale,
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportProgress {
    stage: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub video_path: String,
    pub poster_path: String,
    pub transcript_path: String,
    pub summary_path: String,
    pub transcription_warning: Option<String>,
}

fn progress(app: &AppHandle, stage: &'static str, message: &'static str) {
    let _ = app.emit("export-progress", ExportProgress { stage, message });
}

fn run(mut command: Command, label: &str) -> Result<(), String> {
    let output = command.output().map_err(|e| format!("{label}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{label}: {}",
            stderr
                .lines()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    Ok(())
}

fn load_cursor(path: &str) -> Vec<CursorPoint> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|p: &CursorPoint| {
            p.x.is_finite()
                && p.y.is_finite()
                && (0.0..=1.0).contains(&p.x)
                && (0.0..=1.0).contains(&p.y)
        })
        .collect()
}

fn focus_points(project: &Project, cursor: &[CursorPoint]) -> Vec<(u64, f64, f64)> {
    let edit = &project.edit;
    let step = ((edit.trim_end_ms - edit.trim_start_ms) / 600).max(500);
    let mut out = Vec::new();
    let mut previous = (0.5, 0.5);
    let mut cursor_index = 0usize;
    let mut t = edit.trim_start_ms;
    while t <= edit.trim_end_ms {
        while cursor_index + 1 < cursor.len() && cursor[cursor_index + 1].t_ms <= t {
            cursor_index += 1;
        }
        let target = cursor
            .get(cursor_index)
            .filter(|p| p.t_ms <= t && t.saturating_sub(p.t_ms) < 1500)
            .map(|p| {
                (
                    (p.x - edit.crop.x) / edit.crop.width,
                    (p.y - edit.crop.y) / edit.crop.height,
                )
            })
            .filter(|(x, y)| (0.0..=1.0).contains(x) && (0.0..=1.0).contains(y))
            .unwrap_or((0.5, 0.5));
        previous = (
            previous.0 + (target.0 - previous.0) * 0.28,
            previous.1 + (target.1 - previous.1) * 0.28,
        );
        out.push((t - edit.trim_start_ms, previous.0, previous.1));
        t = t.saturating_add(step);
        if t == u64::MAX {
            break;
        }
    }
    if out.len() == 1 {
        out.push((
            edit.trim_end_ms - edit.trim_start_ms,
            previous.0,
            previous.1,
        ));
    }
    out
}

fn smooth_value(points: &[(u64, f64, f64)], t: u64, axis: usize) -> f64 {
    let pair = points
        .windows(2)
        .find(|p| p[0].0 <= t && p[1].0 >= t)
        .unwrap_or_else(|| &points[points.len() - 2..]);
    let a = if axis == 0 { pair[0].1 } else { pair[0].2 };
    let b = if axis == 0 { pair[1].1 } else { pair[1].2 };
    let fraction = (t.saturating_sub(pair[0].0) as f64 / (pair[1].0 - pair[0].0).max(1) as f64)
        .clamp(0.0, 1.0);
    a + (b - a) * fraction
}

fn axis_expression(points: &[(u64, f64, f64)], axis: usize, zoom: f64) -> String {
    let min_center = 0.5 / zoom;
    let max_center = 1.0 - min_center;
    let coord = |p: &(u64, f64, f64)| if axis == 0 { p.1 } else { p.2 };
    let mut expression = format!(
        "{:.6}",
        (coord(points.last().unwrap()).clamp(min_center, max_center) - min_center) * zoom
    );
    for pair in points.windows(2).rev() {
        let n0 = pair[0].0 as f64 * 30.0 / 1000.0;
        let n1 = pair[1].0 as f64 * 30.0 / 1000.0;
        let v0 = (coord(&pair[0]).clamp(min_center, max_center) - min_center) * zoom;
        let v1 = (coord(&pair[1]).clamp(min_center, max_center) - min_center) * zoom;
        expression = format!(
            "if(lt(on,{n1:.3}),{v0:.6}+({:.6})*(on-{n0:.3})/{:.3},{expression})",
            v1 - v0,
            (n1 - n0).max(1.0)
        );
    }
    // zoompan x/y are source pixel offsets, so scale normalized offsets by input size.
    format!(
        "({expression})*{}",
        if axis == 0 { "iw/zoom" } else { "ih/zoom" }
    )
}

fn ass_time(ms: u64) -> String {
    format!(
        "{}:{:02}:{:02}.{:02}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms / 10 % 100
    )
}
fn escape_ass(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('\n', "\\N")
}

fn write_ass(
    path: &Path,
    project: &Project,
    cursor: &[CursorPoint],
    focus: &[(u64, f64, f64)],
    inner: (u32, u32),
) -> Result<(), String> {
    let edit = &project.edit;
    let mut script = String::from(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Callout,Arial,40,&H00FFFFFF,&H00FFFFFF,&H00202020,&H90000000,0,0,0,0,100,100,0,0,1,3,1,5,20,20,20,1\nStyle: Cursor,Arial,28,&H0000E8FF,&H0000E8FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,5,0,0,0,1\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
    );
    for Callout {
        start_ms,
        end_ms,
        text,
        x,
        y,
    } in &edit.callouts
    {
        let start = start_ms.max(&edit.trim_start_ms);
        let end = end_ms.min(&edit.trim_end_ms);
        if start < end {
            script.push_str(&format!(
                "Dialogue: 1,{},{},Callout,,0,0,0,,{{\\pos({},{})}}{}\n",
                ass_time(start - edit.trim_start_ms),
                ass_time(end - edit.trim_start_ms),
                (x * 1920.0) as i32,
                (y * 1080.0) as i32,
                escape_ass(text)
            ));
        }
    }
    if edit.cursor_highlight {
        let zoom = edit.zoom_strength;
        let left = (1920 - inner.0) / 2;
        let top = (1080 - inner.1) / 2;
        let mut last = 0;
        for point in cursor
            .iter()
            .filter(|p| p.t_ms >= edit.trim_start_ms && p.t_ms < edit.trim_end_ms)
        {
            if point.t_ms < last + 90 {
                continue;
            }
            last = point.t_ms;
            let t = point.t_ms - edit.trim_start_ms;
            let cx = smooth_value(focus, t, 0).clamp(0.5 / zoom, 1.0 - 0.5 / zoom);
            let cy = smooth_value(focus, t, 1).clamp(0.5 / zoom, 1.0 - 0.5 / zoom);
            let px = (point.x - edit.crop.x) / edit.crop.width;
            let py = (point.y - edit.crop.y) / edit.crop.height;
            let x = left as f64 + (px - (cx - 0.5 / zoom)) * zoom * inner.0 as f64;
            let y = top as f64 + (py - (cy - 0.5 / zoom)) * zoom * inner.1 as f64;
            if !(0.0..1920.0).contains(&x) || !(0.0..1080.0).contains(&y) {
                continue;
            }
            script.push_str(&format!("Dialogue: 2,{},{},Cursor,,0,0,0,,{{\\pos({},{})\\p1\\bord5\\1a&HFF&\\3a&H00&\\3c&H00E8FF&}}m 0 -22 b 12 -22 22 -12 22 0 b 22 12 12 22 0 22 b -12 22 -22 12 -22 0 b -22 -12 -12 -22 0 -22\n",
                ass_time(t), ass_time((t+110).min(edit.trim_end_ms-edit.trim_start_ms)), x as i32, y as i32));
        }
    }
    fs::write(path, script).map_err(|e| e.to_string())
}

fn filter_script(project: &Project, focus: &[(u64, f64, f64)], ass: &Path) -> String {
    let edit = &project.edit;
    let inner_w = ((1920.0 * (1.0 - edit.padding * 2.0)).round() as u32 / 2) * 2;
    let inner_h = ((1080.0 * (1.0 - edit.padding * 2.0)).round() as u32 / 2) * 2;
    let crop = &edit.crop;
    let x = axis_expression(focus, 0, edit.zoom_strength);
    let y = axis_expression(focus, 1, edit.zoom_strength);
    // Path is escaped for ffmpeg's filter parser; user text is stored in the ASS file.
    let ass_path = ass
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'");
    format!(
        "[0:v]fps=30,crop=trunc(iw*{:.8}/2)*2:trunc(ih*{:.8}/2)*2:iw*{:.8}:ih*{:.8},zoompan=z='{}':x='{}':y='{}':d=1:s={}x{}:fps=30,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=0x16191c,ass='{}'[v]",
        crop.width,
        crop.height,
        crop.x,
        crop.y,
        edit.zoom_strength,
        x,
        y,
        inner_w,
        inner_h,
        ass_path
    )
}

fn unique_stem(directory: &Path, title: &str, ext: &str) -> PathBuf {
    let base = safe_filename(title);
    for number in 0..10_000 {
        let stem = if number == 0 {
            base.clone()
        } else {
            format!("{base}-{number}")
        };
        if !directory.join(format!("{stem}.{ext}")).exists() {
            return directory.join(stem);
        }
    }
    directory.join(uuid::Uuid::new_v4().to_string())
}

#[tauri::command]
pub fn export_project(
    app: AppHandle,
    db: State<'_, Database>,
    id: String,
    directory: Option<String>,
    locale: UiLocale,
) -> Result<ExportResult, String> {
    let project = db.get(&id)?;
    project.edit.validate(project.duration_ms)?;
    if !Path::new(&project.original_path).is_file() {
        return Err(
            "Original recording is missing. Restore it from backup before exporting.".into(),
        );
    }
    let target = directory.map(PathBuf::from).unwrap_or_else(|| {
        if cfg!(feature = "e2e") {
            if let Some(path) = std::env::var_os("SCREENBOX_TEST_EXPORT_DIR") {
                return PathBuf::from(path);
            }
        }
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| db.root.clone())
            .join("Movies/Screenbox/Exports")
    });
    fs::create_dir_all(&target).map_err(|e| format!("Cannot create export directory: {e}"))?;
    let extension = if project.edit.preset == "h264" {
        "mp4"
    } else {
        "webm"
    };
    let stem = unique_stem(&target, &project.title, extension);
    let video = stem.with_extension(extension);
    let partial = stem.with_extension(format!("partial.{extension}"));
    let poster = stem.with_extension("poster.jpg");
    let transcript = stem.with_extension("transcript.md");
    let summary = stem.with_extension("summary.md");
    let temp = db
        .root
        .join("render")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    let cursor = load_cursor(&project.cursor_path);
    let focus = if project.edit.auto_zoom && !cursor.is_empty() {
        focus_points(&project, &cursor)
    } else {
        vec![
            (0, 0.5, 0.5),
            (
                project.edit.trim_end_ms - project.edit.trim_start_ms,
                0.5,
                0.5,
            ),
        ]
    };
    let inner = (
        ((1920.0 * (1.0 - project.edit.padding * 2.0)).round() as u32 / 2) * 2,
        ((1080.0 * (1.0 - project.edit.padding * 2.0)).round() as u32 / 2) * 2,
    );
    let ass = temp.join("overlay.ass");
    write_ass(&ass, &project, &cursor, &focus, inner)?;
    let filter = temp.join("filter.txt");
    fs::write(&filter, filter_script(&project, &focus, &ass)).map_err(|e| e.to_string())?;
    progress(&app, "render", "Rendering video locally");
    let ffmpeg = crate::model::media_tool("ffmpeg");
    let mut command = Command::new(&ffmpeg);
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-ss",
            &(project.edit.trim_start_ms as f64 / 1000.0).to_string(),
            "-t",
            &((project.edit.trim_end_ms - project.edit.trim_start_ms) as f64 / 1000.0).to_string(),
            "-i",
            &project.original_path,
            "-filter_complex_script",
        ])
        .arg(&filter)
        .args(["-map", "[v]", "-map", "0:a?", "-pix_fmt", "yuv420p"]);
    if extension == "mp4" {
        command.args([
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "18",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
        ]);
    } else {
        command.args([
            "-c:v",
            "libvpx-vp9",
            "-deadline",
            "good",
            "-cpu-used",
            "2",
            "-crf",
            "30",
            "-b:v",
            "0",
            "-c:a",
            "libopus",
            "-b:a",
            "128k",
            "-f",
            "webm",
        ]);
    }
    command.arg(&partial);
    if let Err(error) = run(command, "Video render failed") {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    fs::rename(&partial, &video).map_err(|e| e.to_string())?;
    progress(&app, "poster", "Creating poster frame");
    let mut poster_command = Command::new(&ffmpeg);
    poster_command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-ss",
            "0.2",
            "-i",
        ])
        .arg(&video)
        .args(["-frames:v", "1", "-q:v", "3"])
        .arg(&poster);
    run(poster_command, "Poster generation failed")?;
    progress(&app, "transcript", "Preparing transcript and summary");
    let warning = crate::transcript::write_sidecars(
        &app,
        &db.root,
        &project,
        &video,
        &transcript,
        &summary,
        &ffmpeg,
        locale,
    );
    db.set_export(&id, &video.to_string_lossy())?;
    let _ = fs::remove_dir_all(temp);
    let result = ExportResult {
        video_path: video.to_string_lossy().to_string(),
        poster_path: poster.to_string_lossy().to_string(),
        transcript_path: transcript.to_string_lossy().to_string(),
        summary_path: summary.to_string_lossy().to_string(),
        transcription_warning: warning,
    };
    progress(&app, "done", "Export complete");
    Ok(result)
}

#[tauri::command]
pub fn reveal_in_folder(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("File no longer exists".into());
    }
    Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn copy_file_path(app: AppHandle, path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("File no longer exists".into());
    }
    app.clipboard().write_text(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_smooths_a_fast_cursor_jump() {
        let project = Project {
            id: "test".into(), title: "test".into(), original_path: String::new(),
            cursor_path: String::new(), duration_ms: 1500, width: 640, height: 360,
            microphone: false, created_at: String::new(), edit: crate::model::EditDocument::default_for(1500),
            last_export_path: None,
        };
        let cursor = vec![
            CursorPoint { t_ms: 0, x: 0.0, y: 0.5 },
            CursorPoint { t_ms: 500, x: 1.0, y: 0.5 },
            CursorPoint { t_ms: 1000, x: 1.0, y: 0.5 },
        ];
        let points = focus_points(&project, &cursor);
        assert!(points.len() >= 3);
        assert!(points.windows(2).all(|pair| (pair[1].1 - pair[0].1).abs() < 0.3));
        assert!(points.iter().all(|point| (0.0..=1.0).contains(&point.1)));
        assert!(axis_expression(&points, 0, 1.5).contains("on"));
    }
    #[test]
    fn ass_escapes_user_text() {
        assert_eq!(escape_ass("{hi}\\"), "\\{hi\\}\\\\");
    }
}
