use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct SpeechEngine {
    enabled: bool,
    koko_binary: Option<PathBuf>,
    model_path: Option<PathBuf>,
    voices_path: Option<PathBuf>,
    ort_dylib_path: Option<PathBuf>,
    voice_style: String,
    speech_speed: f32,
}

impl SpeechEngine {
    pub fn new(
        enabled: bool,
        koko_binary: Option<PathBuf>,
        model_path: Option<PathBuf>,
        voices_path: Option<PathBuf>,
        ort_dylib_path: Option<PathBuf>,
        voice_style: String,
        speech_speed: f32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if enabled {
            let koko_binary = koko_binary
                .as_ref()
                .ok_or("speech is enabled but KUCHO_KOKO_BINARY is not configured")?;

            let model_path = model_path
                .as_ref()
                .ok_or("speech is enabled but KUCHO_KOKORO_MODEL is not configured")?;

            let voices_path = voices_path
                .as_ref()
                .ok_or("speech is enabled but KUCHO_KOKORO_VOICES is not configured")?;

            if !koko_binary.exists() {
                return Err(
                    format!("Kokoros binary does not exist: {}", koko_binary.display()).into(),
                );
            }

            if !model_path.exists() {
                return Err(
                    format!("Kokoro model does not exist: {}", model_path.display()).into(),
                );
            }

            if !voices_path.exists() {
                return Err(format!(
                    "Kokoro voices file does not exist: {}",
                    voices_path.display()
                )
                .into());
            }

            if let Some(ref dylib) = ort_dylib_path {
                if !dylib.exists() {
                    return Err(format!(
                        "ONNX Runtime library does not exist: {}",
                        dylib.display()
                    )
                    .into());
                }
            }
        }

        Ok(Self {
            enabled,
            koko_binary,
            model_path,
            voices_path,
            ort_dylib_path,
            voice_style,
            speech_speed,
        })
    }

    pub async fn speak(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            return Ok(());
        }

        let koko_binary = self
            .koko_binary
            .as_ref()
            .ok_or("Kokoros binary is not configured")?;

        let model_path = self
            .model_path
            .as_ref()
            .ok_or("Kokoro model is not configured")?;

        let voices_path = self
            .voices_path
            .as_ref()
            .ok_or("Kokoro voices file is not configured")?;

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("kucho-speech.wav");

        let mut command = Command::new(koko_binary);

        command
            .arg("--model")
            .arg(model_path)
            .arg("--data")
            .arg(voices_path)
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

        let status = command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;

        if !status.success() {
            return Err(format!("Kokoros exited with status {status}").into());
        }

        play_audio(&output_path).await?;

        let _ = tokio::fs::remove_file(&output_path).await;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SpeechHandle {
    tx: mpsc::Sender<String>,
}

impl SpeechHandle {
    pub async fn speak(&self, text: String) -> Result<(), Box<dyn std::error::Error>> {
        self.tx
            .send(text)
            .await
            .map_err(|_| "speech queue is unavailable")?;

        Ok(())
    }
}

pub fn start_speech_worker(engine: SpeechEngine) -> SpeechHandle {
    let (tx, mut rx) = mpsc::channel::<String>(4);

    tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            if let Err(err) = engine.speak(&text).await {
                eprintln!("[Kūchō Speech Error]: {err}");
            }
        }
    });

    SpeechHandle { tx }
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
