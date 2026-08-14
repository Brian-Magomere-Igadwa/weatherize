use crate::mcp::{get_mcp_tool_definitions, JsonRpcRequest, JsonRpcResponse};
use crate::state::TelemetrySample;
use actix_web::{get, post, web, HttpResponse, Responder};

use crate::history::TelemetryHistory;
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};

use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use weather_core::EnvironmentEvent;

use crate::analysis::analyze_samples;
use std::time::Duration;

pub struct AppState {
    pub telemetry_rx: watch::Receiver<Option<TelemetrySample>>,
    pub telemetry_history: Arc<RwLock<TelemetryHistory>>,
    pub environment_event_tx: broadcast::Sender<EnvironmentEvent>,
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

#[get("/api/v1/events")]
pub async fn stream_environment_events(data: web::Data<AppState>) -> impl Responder {
    let rx = data.environment_event_tx.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let json = serde_json::to_string(&event).ok()?;

            Some(Ok::<_, actix_web::Error>(web::Bytes::from(format!(
                "data: {json}\n\n"
            ))))
        }

        Err(_) => None,
    });

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(stream)
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

            match tool_name {
                Some("get_indoor_climate") => {
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

                    let result = serde_json::json!({
                        "content": content
                    });

                    HttpResponse::Ok().json(JsonRpcResponse::success(request.id, result))
                }

                Some("get_climate_trend") => {
                    let window_seconds = request
                        .params
                        .get("arguments")
                        .and_then(|args| args.get("window_seconds"))
                        .and_then(|value| value.as_u64());

                    let Some(window_seconds) = window_seconds else {
                        let result = serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": "window_seconds must be provided as a positive integer."
                            }]
                        });

                        return HttpResponse::Ok()
                            .json(JsonRpcResponse::success(request.id, result));
                    };

                    if window_seconds == 0 || window_seconds > 300 {
                        let result = serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": "window_seconds must be between 1 and 300."
                            }]
                        });

                        return HttpResponse::Ok()
                            .json(JsonRpcResponse::success(request.id, result));
                    }

                    let samples = {
                        let history = data.telemetry_history.read().await;
                        history.recent(Duration::from_secs(window_seconds))
                    };

                    let content = match analyze_samples(&samples) {
                        Ok(trend) => serde_json::json!([{
                            "type": "text",
                            "text": serde_json::to_string(&trend).unwrap_or_default()
                        }]),

                        Err(_) => serde_json::json!([{
                            "type": "text",
                            "text": "Not enough telemetry has been collected for that time window yet."
                        }]),
                    };

                    let result = serde_json::json!({
                        "content": content
                    });

                    HttpResponse::Ok().json(JsonRpcResponse::success(request.id, result))
                }

                _ => HttpResponse::Ok().json(JsonRpcResponse::method_not_found(
                    request.id,
                    tool_name.unwrap_or("unknown"),
                )),
            }
        }
        _ => HttpResponse::Ok().json(JsonRpcResponse::method_not_found(
            request.id,
            &request.method,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weather_core::{EnvironmentEvent, SafetyStatus};

    #[test]
    fn formats_environment_event_as_sse() {
        let event = EnvironmentEvent::SafetyStatusChanged {
            from: SafetyStatus::Optimal,
            to: SafetyStatus::Stuffy,
        };

        let json = serde_json::to_string(&event).unwrap();
        let text = format!("data: {json}\n\n");

        assert!(text.starts_with("data: "));
        assert!(text.contains("SAFETY_STATUS_CHANGED"));
        assert!(text.contains("STUFFY"));
        assert!(text.ends_with("\n\n"));
    }
}
