mod agent;
mod events;
mod speech;

use agent::AgentEngine;
use clap::Parser;
use speech::SpeechEngine;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Kūchō Interactive Ollama CLI Agent with Dynamic MCP Hardware Tools"
)]
struct Args {
    #[arg(
        long,
        env = "KUCHO_MCP_URL",
        default_value = "http://127.0.0.1:8080/mcp"
    )]
    mcp_url: String,

    #[arg(
        long,
        env = "KUCHO_OLLAMA_URL",
        default_value = "http://127.0.0.1:11434"
    )]
    ollama_url: String,

    #[arg(short, long, env = "KUCHO_CHAT_MODEL", default_value = "qwen3:4b")]
    model: String,

    #[arg(long, env = "KUCHO_COMMENTARY_MODEL", default_value = "qwen2.5:3b")]
    commentary_model: String,

    #[arg(long, env = "KUCHO_SPEECH", default_value_t = false)]
    speech: bool,

    #[arg(long, env = "KUCHO_KOKO_BINARY")]
    koko_binary: Option<PathBuf>,

    #[arg(long, env = "KUCHO_KOKORO_MODEL")]
    kokoro_model: Option<PathBuf>,

    #[arg(long, env = "KUCHO_KOKORO_VOICES")]
    kokoro_voices: Option<PathBuf>,

    #[arg(long, env = "ORT_DYLIB_PATH")]
    ort_dylib: Option<PathBuf>,

    #[arg(long, env = "KUCHO_VOICE_STYLE", default_value = "bm_lewis")]
    voice_style: String,

    #[arg(long, env = "KUCHO_SPEECH_SPEED", default_value_t = 0.92)]
    speech_speed: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let speech = SpeechEngine::new(
        args.speech,
        args.koko_binary,
        args.kokoro_model,
        args.kokoro_voices,
        args.ort_dylib,
        args.voice_style,
        args.speech_speed,
    )?;

    let agent = AgentEngine::new(
        args.mcp_url,
        args.ollama_url,
        args.model,
        args.commentary_model,
        speech,
    );

    agent.run_repl().await?;

    Ok(())
}
