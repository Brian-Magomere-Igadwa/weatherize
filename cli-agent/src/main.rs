mod agent;
mod events;
mod speech;

use agent::AgentEngine;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Kūchō Interactive Ollama CLI Agent with Dynamic MCP Hardware Tools"
)]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:8080/mcp")]
    mcp_url: String,

    #[arg(long, default_value = "http://127.0.0.1:11434")]
    ollama_url: String,

    #[arg(short, long, default_value = "qwen3:4b")]
    model: String,

    #[arg(long, default_value = "qwen2.5:3b")]
    commentary_model: String,

    #[arg(long, default_value_t = true)]
    speech: bool,

    #[arg(
        long,
        default_value = "/Users/user/Desktop/Kokoros/target/release/koko"
    )]
    koko_binary: PathBuf,

    #[arg(
        long,
        default_value = "/Users/user/Desktop/Kokoros/checkpoints/kokoro-v1.0.onnx"
    )]
    kokoro_model: PathBuf,

    #[arg(
        long,
        default_value = "/Users/user/Desktop/Kokoros/data/voices-v1.0.bin"
    )]
    kokoro_voices: PathBuf,

    #[arg(
        long,
        default_value = "/Users/user/Desktop/onnxruntime-osx-x86_64-1.23.2/lib/libonnxruntime.dylib"
    )]
    ort_dylib: PathBuf,

    #[arg(long, default_value = "bm_lewis")]
    voice_style: String,

    #[arg(long, default_value_t = 0.92)]
    speech_speed: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let agent = AgentEngine::new(
        args.mcp_url,
        args.ollama_url,
        args.model,
        args.commentary_model,
        args.speech,
        args.koko_binary,
        args.kokoro_model,
        args.kokoro_voices,
        Some(args.ort_dylib),
        args.voice_style,
        args.speech_speed,
    );

    agent.run_repl().await?;

    Ok(())
}
