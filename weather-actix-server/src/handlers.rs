use crate::mcp::{get_mcp_tool_definitions, JsonRpcRequest, JsonRpcResponse};
use crate::state::TelemetrySample;
use actix_web::{get, post, web, HttpResponse, Responder};
use tokio::sync::watch;

pub struct AppState {
    pub telemetry_rx: watch::Receiver<Option<TelemetrySample>>,
}

#[get("/health")]
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "UP" }))
}

#[get("/api/v1/telemetry")]
pub async fn get_current_telemetry(data: web::Data<AppState>) -> impl Responder {
    let current = data.telemetry_rx.borrow();
    match *current {
        Some(sample) => HttpResponse::Ok().json(sample.payload),
        None => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": "No telemetry payload received from board yet"
        })),
    }
}

#[post("/mcp")]
pub async fn handle_mcp_rpc(
    data: web::Data<AppState>,
    req: web::Json<JsonRpcRequest>,
) -> impl Responder {
    let request = req.into_inner();

    match request.method.as_str() {
        "tools/list" => {
            let tools = get_mcp_tool_definitions();
            HttpResponse::Ok().json(JsonRpcResponse::success(request.id, tools))
        }
        "tools/call" => {
            let tool_name = request.params.get("name").and_then(|v| v.as_str());
            if tool_name == Some("get_indoor_climate") {
                let current = data.telemetry_rx.borrow();
                let content = match *current {
                    Some(sample) => serde_json::json!([{
                        "type": "text",
                        "text": serde_json::to_string(&sample.payload).unwrap_or_default()
                    }]),
                    None => serde_json::json!([{
                        "type": "text",
                        "text": "Sensor reading unavailable. Hardware station warming up or disconnected."
                    }]),
                };

                let result = serde_json::json!({ "content": content });
                HttpResponse::Ok().json(JsonRpcResponse::success(request.id, result))
            } else {
                HttpResponse::Ok().json(JsonRpcResponse::method_not_found(
                    request.id,
                    tool_name.unwrap_or("unknown"),
                ))
            }
        }
        _ => HttpResponse::Ok().json(JsonRpcResponse::method_not_found(
            request.id,
            &request.method,
        )),
    }
}
