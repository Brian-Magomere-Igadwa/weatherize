use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct SpeechEngine {
    enabled: bool,
    koko_binary: PathBuf,
    model_path: PathBuf,
    voices_path: PathBuf,
    ort_dylib_path: Option<PathBuf>,
    voice_style: String,
    speech_speed: f32,
}

impl SpeechEngine {
    pub fn new(
        enabled: bool,
        koko_binary: PathBuf,
        model_path: PathBuf,
        voices_path: PathBuf,
        ort_dylib_path: Option<PathBuf>,
        voice_style: String,
        speech_speed: f32,
    ) -> Self {
        Self {
            enabled,
            koko_binary,
            model_path,
            voices_path,
            ort_dylib_path,
            voice_style,
            speech_speed,
        }
    }

    pub async fn speak(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            return Ok(());
        }

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("kucho-speech.wav");

        let mut command = Command::new(&self.koko_binary);

        command
            .arg("--model")
            .arg(&self.model_path)
            .arg("--data")
            .arg(&self.voices_path)
            .arg("--style")
            .arg(&self.voice_style)
            .arg("--speed")
            .arg(self.speech_speed.to_string())
            .arg("text")
            .arg(text)
            .arg("--output")
            .arg(&output_path);

        if let Some(ref dylib) = self.ort_dylib_path {
            command.env("ORT_DYLIB_PATH", dylib);
        }

        let status = command.status().await?;

        if !status.success() {
            return Err(format!("Kokoros exited with status {status}").into());
        }

        play_audio(&output_path).await?;

        let _ = tokio::fs::remove_file(&output_path).await;

        Ok(())
    }
}

async fn play_audio(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    let player = "afplay";

    #[cfg(target_os = "linux")]
    let player = "aplay";

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        return Err("unsupported audio playback platform".into());
    }

    let status = Command::new(player).arg(path).status().await?;

    if !status.success() {
        return Err(format!("audio player exited with status {status}").into());
    }

    Ok(())
}
