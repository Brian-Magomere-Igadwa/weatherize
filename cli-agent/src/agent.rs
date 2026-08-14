use crate::speech::SpeechEngine;

use crate::events::AgentEvent;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tokio_util::io::StreamReader;
use weather_core::EnvironmentEvent;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    tools: Vec<Value>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    think: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCall {
    pub function: FunctionCall,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    message: Message,
}

#[derive(Clone)]
pub struct AgentEngine {
    client: reqwest::Client,
    mcp_url: String,
    ollama_url: String,
    model: String,
    commentary_model: String,
    speech: SpeechEngine,
}

impl AgentEngine {
    pub fn new(
        mcp_url: String,
        ollama_url: String,
        model: String,
        commentary_model: String,
        speech_enabled: bool,
        koko_binary: PathBuf,
        kokoro_model: PathBuf,
        kokoro_voices: PathBuf,
        ort_dylib: Option<PathBuf>,
        voice_style: String,
        speech_speed: f32,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            mcp_url,
            ollama_url,
            model,
            commentary_model,
            speech: SpeechEngine::new(
                speech_enabled,
                koko_binary,
                kokoro_model,
                kokoro_voices,
                ort_dylib,
                voice_style,
                speech_speed,
            ),
        }
    }

    /// Discover available tools dynamically from the MCP server via JSON-RPC.
    async fn fetch_mcp_tools(&self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let tools_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });

        let tools_res: Value = self
            .client
            .post(&self.mcp_url)
            .json(&tools_req)
            .send()
            .await?
            .json()
            .await?;

        let tools = tools_res["result"]["tools"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        // Convert MCP tool schemas to Ollama function tool format
        let ollama_tools = tools
            .into_iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t["name"],
                        "description": t["description"],
                        "parameters": t["inputSchema"]
                    }
                })
            })
            .collect();

        Ok(ollama_tools)
    }

    /// Execute a tool call against the MCP server via JSON-RPC.
    async fn call_mcp_tool(
        &self,
        name: &str,
        args: &Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let execute_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": args
            }
        });

        let execute_res: Value = self
            .client
            .post(&self.mcp_url)
            .json(&execute_req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(text) = execute_res["result"]["content"][0]["text"].as_str() {
            Ok(text.to_string())
        } else {
            Ok(execute_res["result"].to_string())
        }
    }

    /// Stream Ollama response NDJSON tokens to stdout line-by-line.
    async fn stream_ollama(
        &self,
        payload: &ChatRequest,
    ) -> Result<(String, Option<Vec<ToolCall>>), Box<dyn std::error::Error>> {
        let endpoint = format!("{}/api/chat", self.ollama_url);
        let res = self.client.post(&endpoint).json(payload).send().await?;

        let byte_stream = res
            .bytes_stream()
            .map(|item| item.map_err(|e| io::Error::new(io::ErrorKind::Other, e)));
        let mut reader = tokio::io::BufReader::new(StreamReader::new(byte_stream));

        let mut line = String::new();
        let mut accumulated_content = String::new();
        let mut accumulated_tool_calls: Vec<ToolCall> = Vec::new();

        while reader.read_line(&mut line).await? > 0 {
            if let Ok(chunk) = serde_json::from_str::<ChatResponse>(&line) {
                if let Some(ref text) = chunk.message.content {
                    print!("{text}");
                    io::stdout().flush()?;
                    accumulated_content.push_str(text);
                }

                if let Some(ref tool_calls) = chunk.message.tool_calls {
                    accumulated_tool_calls.extend(tool_calls.clone());
                }
            }
            line.clear();
        }

        let tool_calls = if accumulated_tool_calls.is_empty() {
            None
        } else {
            Some(accumulated_tool_calls)
        };

        Ok((accumulated_content, tool_calls))
    }

    async fn handle_user_message(
        &self,
        input: String,
        messages: &mut Vec<Message>,
        tools: &[Value],
    ) -> Result<(), Box<dyn std::error::Error>> {
        messages.push(Message {
            role: "user".to_string(),
            content: Some(input),
            tool_calls: None,
        });

        let payload = ChatRequest {
            model: self.model.clone(),
            messages: messages.clone(),
            tools: tools.to_vec(),
            stream: true,
            think: None,
        };
        print!("\n");

        let (content, tool_calls) = match self.stream_ollama(&payload).await {
            Ok(res) => res,
            Err(e) => {
                eprintln!("\n[ERROR] Request failed: {e}\n");
                return Ok(());
            }
        };

        if let Some(calls) = tool_calls {
            messages.push(Message {
                role: "assistant".to_string(),
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                tool_calls: Some(calls.clone()),
            });

            for call in calls {
                let fn_name = &call.function.name;
                let fn_args = &call.function.arguments;

                println!("\n[Kūchō Agent: Calling MCP Tool '{fn_name}']");

                match self.call_mcp_tool(fn_name, fn_args).await {
                    Ok(output) => {
                        println!("[MCP Output]: {output}\n");

                        messages.push(Message {
                            role: "tool".to_string(),
                            content: Some(output),
                            tool_calls: None,
                        });
                    }

                    Err(e) => {
                        eprintln!("[MCP Tool Error]: {e}");

                        messages.push(Message {
                            role: "tool".to_string(),
                            content: Some(format!("Error executing tool: {e}")),
                            tool_calls: None,
                        });
                    }
                }
            }

            let final_payload = ChatRequest {
                model: self.model.clone(),
                messages: messages.clone(),
                tools: tools.to_vec(),
                stream: true,
                think: None,
            };

            let (final_content, _) = match self.stream_ollama(&final_payload).await {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("\n[ERROR] Synthesis stream failed: {e}");
                    return Ok(());
                }
            };

            messages.push(Message {
                role: "assistant".to_string(),
                content: Some(final_content),
                tool_calls: None,
            });

            println!("\n");
        } else {
            messages.push(Message {
                role: "assistant".to_string(),
                content: Some(content),
                tool_calls: None,
            });

            println!("\n");
        }

        Ok(())
    }

    async fn listen_for_environment_events(
        &self,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint = format!("{}/api/v1/events", self.mcp_url.trim_end_matches("/mcp"));

        let response = self.client.get(&endpoint).send().await?;

        if !response.status().is_success() {
            return Err(format!("environment event stream returned {}", response.status()).into());
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(boundary) = buffer.find("\n\n") {
                let event_block = buffer[..boundary].to_string();
                buffer.drain(..boundary + 2);

                for line in event_block.lines() {
                    if let Some(json) = line.strip_prefix("data: ") {
                        match serde_json::from_str::<EnvironmentEvent>(json) {
                            Ok(event) => {
                                if event_tx.send(AgentEvent::Environment(event)).await.is_err() {
                                    return Ok(());
                                }
                            }

                            Err(err) => {
                                eprintln!("[Kūchō Environment Event Parse Error]: {err}");
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn generate_environment_commentary(
        &self,
        event: &EnvironmentEvent,
    ) -> Result<String, Box<dyn std::error::Error>> {
        println!("\n[generate_environment_commentary started...]");
        let event_json = serde_json::to_string(event)?;

        let messages = vec![
        Message {
            role: "system".to_string(),
            content: Some(
                "You are Kūchō, a local ambient assistant observing indoor climate conditions. \
                An environmental change has already been determined to be significant by deterministic monitoring logic. \
                Your only job is to comment on it naturally in one short sentence. \
                Your personality is dry, sarcastic, slightly grumpy, clever, and mildly dramatic. \
                You can tease the user and sound exasperated, but never become cruel, hostile, or genuinely insulting. \
                Prefer witty observations over generic alerts. \
                Do not explain your reasoning. \
                Do not dump raw JSON or technical field names. \
                Do not call tools. \
                Keep it concise enough to be spoken aloud."
                    .to_string(),
            ),
            tool_calls: None,
        },
        Message {
            role: "user".to_string(),
            content: Some(format!(
                "/no_think\nDescribe this detected environmental event to the user:\n{event_json}"
            )),
            tool_calls: None,
        },
    ];

        let payload = ChatRequest {
            model: self.commentary_model.clone(),
            messages,
            tools: vec![],
            stream: true,
            think: Some(false),
        };

        let (content, _) = self.stream_ollama(&payload).await?;

        Ok(content)
    }
    async fn run_event_loop(
        &self,
        mut event_rx: mpsc::Receiver<AgentEvent>,
        ready_tx: mpsc::Sender<()>,
        tools: Vec<Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut messages: Vec<Message> = vec![Message {
        role: "system".to_string(),
        content: Some(
    "You are Kūchō, a local AI assistant connected to live physical hardware sensors via MCP tools. \
     Use get_indoor_climate for questions about current conditions right now. \
     Use get_climate_trend for questions about how temperature or humidity changed over time, \
     including whether values are rising, falling, increasing, decreasing, or trending."
        .to_string(),
),
        tool_calls: None,
    }];

        while let Some(event) = event_rx.recv().await {
            match event {
                AgentEvent::UserMessage(input) => {
                    println!("\n[Kūchō is working...]\n");

                    self.handle_user_message(input, &mut messages, &tools)
                        .await?;

                    let _ = ready_tx.send(()).await;
                }
                AgentEvent::Environment(event) => {
                    println!("\n[Kūchō noticed something...]");

                    match self.generate_environment_commentary(&event).await {
                        Ok(commentary) => {
                            println!("\n");

                            let speech = self.speech.clone();
                            let speech_text = commentary.clone();

                            tokio::spawn(async move {
                                if let Err(err) = speech.speak(&speech_text).await {
                                    eprintln!("[Kūchō Speech Error]: {err}");
                                }
                            });

                            messages.push(Message {
                                role: "assistant".to_string(),
                                content: Some(commentary),
                                tool_calls: None,
                            });
                        }

                        Err(err) => {
                            eprintln!("[Kūchō Commentary Error]: {err}");

                            println!("[Environmental event: {event:?}]");
                        }
                    }
                }
                AgentEvent::Shutdown => {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Interactive REPL terminal session.
    pub async fn run_repl(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Connecting to MCP server at {}...", self.mcp_url);
        let tools = match self.fetch_mcp_tools().await {
            Ok(t) => {
                println!("Loaded {} MCP tool(s) successfully.\n", t.len());
                t
            }
            Err(e) => {
                eprintln!("Warning: Could not fetch MCP tools ({e}). Proceeding without tools.\n");
                vec![]
            }
        };

        println!("==================================================");
        println!("  Kūchō Interactive CLI (Ollama Streaming + MCP)  ");
        println!("  Type 'exit' or 'quit' to end session.           ");
        println!("==================================================\n");
        let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(32);
        let (ready_tx, mut ready_rx) = mpsc::channel::<()>(1);
        ready_tx.send(()).await?;

        let environment_event_tx = event_tx.clone();
        let environment_listener = self.clone();

        tokio::spawn(async move {
            if let Err(err) = environment_listener
                .listen_for_environment_events(environment_event_tx)
                .await
            {
                eprintln!("[Kūchō Environment Listener Error]: {err}");
            }
        });

        let input_tx = event_tx.clone();
        let input_ready_tx = ready_tx.clone();

        tokio::spawn(async move {
            while ready_rx.recv().await.is_some() {
                print!("> ");

                if io::stdout().flush().is_err() {
                    break;
                }

                let mut input = String::new();

                match io::stdin().read_line(&mut input) {
                    Ok(0) => {
                        let _ = input_tx.send(AgentEvent::Shutdown).await;
                        break;
                    }
                    Ok(_) => {}
                    Err(_) => {
                        let _ = input_tx.send(AgentEvent::Shutdown).await;
                        break;
                    }
                }

                let input = input.trim();

                if input.is_empty() {
                    let _ = input_ready_tx.send(()).await;
                    continue;
                }

                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    let _ = input_tx.send(AgentEvent::Shutdown).await;
                    break;
                }

                if input_tx
                    .send(AgentEvent::UserMessage(input.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }

                // IMPORTANT:
                // Do not request another input here.
                //
                // run_event_loop() will send READY only after
                // handle_user_message() has completely finished.
            }
        });
        self.run_event_loop(event_rx, ready_tx, tools).await?;

        println!("Bye!");

        Ok(())
    }
}
