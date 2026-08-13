use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn method_not_found(id: Option<Value>, method: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(serde_json::json!({
                "code": -32601,
                "message": format!("Method not found: {}", method)
            })),
        }
    }
}

pub fn get_mcp_tool_definitions() -> Value {
    serde_json::json!({
        "tools": [
            {
                "name": "get_indoor_climate",
                "description": "Get the latest current indoor temperature, humidity, and safety status. Use this only for questions about what conditions are like right now. Do not use this for questions asking whether temperature or humidity has risen, fallen, changed, or trended over time.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "get_climate_trend",
                "description": "Analyze how indoor temperature and humidity changed over a recent time window. Use this for questions asking whether conditions have risen, fallen, changed, increased, decreased, or trended over time.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "window_seconds": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 300,
                            "description": "How many recent seconds of telemetry to analyze"
                        }
                    },
                    "required": ["window_seconds"]
                }
            }
        ]
    })
}
