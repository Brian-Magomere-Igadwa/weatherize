mod agent;
mod events;
mod speech;

use agent::AgentEngine;
use clap::Parser;

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
    );

    agent.run_repl().await?;

    Ok(())
}
