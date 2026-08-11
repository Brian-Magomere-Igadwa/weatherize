use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use tokio_util::io::StreamReader;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    tools: Vec<Value>,
    stream: bool,
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

pub struct AgentEngine {
    client: reqwest::Client,
    mcp_url: String,
    ollama_url: String,
    model: String,
}

impl AgentEngine {
    pub fn new(mcp_url: String, ollama_url: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            mcp_url,
            ollama_url,
            model,
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

        let mut messages: Vec<Message> = vec![Message {
            role: "system".to_string(),
            content: Some(
                "You are Kūchō, a local AI assistant connected to live physical hardware sensors via MCP tools. Call available tools when environmental data is requested."
                    .to_string(),
            ),
            tool_calls: None,
        }];

        println!("==================================================");
        println!("  Kūchō Interactive CLI (Ollama Streaming + MCP)  ");
        println!("  Type 'exit' or 'quit' to end session.           ");
        println!("==================================================\n");

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            if io::stdin().read_line(&mut input)? == 0 {
                break;
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                println!("Bye!");
                break;
            }

            messages.push(Message {
                role: "user".to_string(),
                content: Some(input.to_string()),
                tool_calls: None,
            });

            let payload = ChatRequest {
                model: self.model.clone(),
                messages: messages.clone(),
                tools: tools.clone(),
                stream: true,
            };

            print!("\n");
            let (content, tool_calls) = match self.stream_ollama(&payload).await {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("\n[ERROR] Request failed: {e}\n");
                    continue;
                }
            };

            // Handle tool execution loop if requested by model
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

                // Secondary streaming pass to synthesize final tool response
                let final_payload = ChatRequest {
                    model: self.model.clone(),
                    messages: messages.clone(),
                    tools: tools.clone(),
                    stream: true,
                };

                let (final_content, _) = match self.stream_ollama(&final_payload).await {
                    Ok(res) => res,
                    Err(e) => {
                        eprintln!("\n[ERROR] Synthesis stream failed: {e}");
                        continue;
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
        }

        Ok(())
    }
}
