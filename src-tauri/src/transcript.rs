use crate::{model::Project, settings::UiLocale};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tauri::{AppHandle, Emitter, Manager};

fn helper_path(app: &AppHandle, name: &str) -> PathBuf {
    let bundled = app
        .path()
        .resource_dir()
        .unwrap_or_default()
        .join("resources")
        .join(name);
    if bundled.is_file() {
        bundled
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(name)
    }
}

fn ensure_model(app: &AppHandle, root: &Path) -> Result<PathBuf, String> {
    let directory = root.join("models");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let model = directory.join("ggml-base.bin");
    if model.metadata().is_ok_and(|m| m.len() > 10_000_000) {
        return Ok(model);
    }
    let partial = directory.join("ggml-base.partial");
    let url = std::env::var("SCREENBOX_WHISPER_MODEL_URL").unwrap_or_else(|_| {
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".into()
    });
    if !url.starts_with("https://") {
        return Err("Model URL must use HTTPS".into());
    }
    let _ = app.emit(
        "model-download",
        serde_json::json!({"state":"started","percent":0}),
    );
    let mut child = Command::new("curl")
        .args(["--fail", "--location", "--progress-bar", "--output"])
        .arg(&partial)
        .arg(url)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stderr) = child.stderr.take() {
        let mut byte = [0u8; 1];
        let mut number = String::new();
        let mut last_percent = 0;
        while stderr.read(&mut byte).unwrap_or(0) == 1 {
            let c = byte[0] as char;
            if c.is_ascii_digit() || c == '.' {
                number.push(c);
                if number.len() > 6 {
                    number.clear();
                }
            } else if c == '%' {
                if let Ok(percent) = number.parse::<f32>().map(|value| value.floor() as u8) {
                    if percent > last_percent && percent <= 100 {
                        last_percent = percent;
                        let _ = app.emit(
                            "model-download",
                            serde_json::json!({"state":"downloading","percent":percent}),
                        );
                    }
                }
                number.clear();
            } else if c == '\r' || c == '\n' || c == ' ' {
                number.clear();
            }
        }
    }
    if !child.wait().map_err(|e| e.to_string())?.success() {
        let _ = fs::remove_file(&partial);
        return Err("Model download failed; check your connection and retry export".into());
    }
    if partial.metadata().map_err(|e| e.to_string())?.len() < 10_000_000 {
        let _ = fs::remove_file(&partial);
        return Err("Downloaded model is incomplete".into());
    }
    fs::rename(partial, &model).map_err(|e| e.to_string())?;
    let _ = app.emit(
        "model-download",
        serde_json::json!({"state":"done","percent":100}),
    );
    Ok(model)
}

fn run(mut command: Command, label: &str) -> Result<(), String> {
    let output = command.output().map_err(|e| format!("{label}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{label}: {}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .last()
                .unwrap_or("failed")
        ));
    }
    Ok(())
}

fn transcript_lines(vtt: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut time = String::new();
    for line in vtt.lines().map(str::trim) {
        if line.contains(" --> ") {
            time = line.split(" --> ").next().unwrap_or("").to_string();
        } else if !line.is_empty()
            && line != "WEBVTT"
            && !line.chars().all(|c| c.is_ascii_digit())
            && !time.is_empty()
        {
            result.push((time.clone(), line.to_string()));
            time.clear();
        }
    }
    result
}

pub fn write_sidecars(
    app: &AppHandle,
    root: &Path,
    project: &Project,
    video: &Path,
    transcript: &Path,
    summary: &Path,
    ffmpeg: &str,
    locale: UiLocale,
) -> Option<String> {
    let duration = (project.edit.trim_end_ms - project.edit.trim_start_ms) as f64 / 1000.0;
    let mut warning = None;
    let mut segments = Vec::new();
    if project.microphone {
        let result = (|| -> Result<(), String> {
            let model = ensure_model(app, root)?;
            let helper = helper_path(app, "whisper-cli");
            if !helper.is_file() {
                return Err("whisper-cli is unavailable; rerun ./dev.sh".into());
            }
            let temp = root
                .join("render")
                .join(format!("transcript-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
            let wav = temp.join("audio.wav");
            let mut convert = Command::new(ffmpeg);
            convert
                .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
                .arg(video)
                .args(["-vn", "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
                .arg(&wav);
            run(convert, "Audio extraction failed")?;
            let output_stem = temp.join("transcript");
            let mut whisper = Command::new(helper);
            whisper
                .arg("-m")
                .arg(model)
                .arg("-f")
                .arg(&wav)
                .arg("-ovtt")
                .arg("-of")
                .arg(&output_stem);
            run(whisper, "Local transcription failed")?;
            segments = transcript_lines(
                &fs::read_to_string(output_stem.with_extension("vtt"))
                    .map_err(|e| e.to_string())?,
            );
            let _ = fs::remove_dir_all(temp);
            Ok(())
        })();
        if let Err(error) = result {
            warning = Some(error);
        }
    }
    let chinese = locale == UiLocale::Chinese;
    let mut transcript_text = if chinese {
        format!("# 转录：{}\n\n", project.title)
    } else {
        format!("# Transcript: {}\n\n", project.title)
    };
    if !project.microphone {
        transcript_text.push_str(if chinese { "没有录制麦克风音频。\n" } else { "No microphone audio was recorded.\n" });
    } else if segments.is_empty() {
        transcript_text.push_str(if chinese { "当前无法生成转录。请根据 Screenbox 中的提示解决问题后重试导出。\n" } else { "Transcription is unavailable. Retry export after resolving the issue shown in Screenbox.\n" });
    } else {
        for (time, text) in &segments {
            transcript_text.push_str(&format!("- **{}** {}\n", time, text));
        }
    }
    if let Err(error) = fs::write(transcript, transcript_text) {
        warning = Some(format!("Transcript file could not be written: {error}"));
    }
    let mut summary_text = if chinese {
        format!("# 摘要：{}\n\n- 时长：{:.1} 秒\n- 视频：{}\n\n", project.title, duration, video.display())
    } else {
        format!("# Summary: {}\n\n- Duration: {:.1} seconds\n- Export: {}\n\n", project.title, duration, video.display())
    };
    if segments.is_empty() {
        summary_text.push_str(if chinese { "没有可用于提取摘要的语音内容。\n" } else { "No spoken content is available for an extractive summary.\n" });
    } else {
        summary_text.push_str(if chinese { "## 关键片段\n\n" } else { "## Key moments\n\n" });
        let step = (segments.len() / 5).max(1);
        for (time, text) in segments.iter().step_by(step).take(5) {
            summary_text.push_str(&format!("- **{}** {}\n", time, text));
        }
    }
    if !project.edit.callouts.is_empty() {
        summary_text.push_str(if chinese { "\n## 文字标注\n\n" } else { "\n## Callouts\n\n" });
        for callout in &project.edit.callouts {
            summary_text.push_str(&format!("- {}\n", callout.text.replace('\n', " ")));
        }
    }
    if let Err(error) = fs::write(summary, summary_text) {
        warning = Some(format!("Summary file could not be written: {error}"));
    }
    warning
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_vtt_segments() {
        let text = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHello.\n";
        assert_eq!(
            transcript_lines(text),
            vec![("00:00:01.000".into(), "Hello.".into())]
        );
    }
}
