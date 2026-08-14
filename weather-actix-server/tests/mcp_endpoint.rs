use actix_web::{test, web, App};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{watch, RwLock};
use weather_actix_server::{handlers, history::TelemetryHistory, state::TelemetrySample};
use weather_core::TelemetryPayload;

#[actix_rt::test]
async fn test_mcp_tools_list_and_call() {
    let (tx, rx) = watch::channel(None);
    let history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(300))));
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx: rx,
                telemetry_history: history,
            }))
            .service(handlers::handle_mcp_rpc),
    )
    .await;

    // 1. Assert tools/list returns get_indoor_climate
    let list_req = test::TestRequest::post()
        .uri("/mcp")
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }))
        .to_request();

    let list_res: serde_json::Value = test::call_and_read_body_json(&app, list_req).await;
    assert_eq!(list_res["result"]["tools"][0]["name"], "get_indoor_climate");
    assert_eq!(list_res["result"]["tools"][1]["name"], "get_climate_trend");

    // 2. Broadcast telemetry state and test tools/call execution
    let payload = TelemetryPayload::from_raw_dht11(26, 2, 62, 0);
    // tx.send(Some(payload)).unwrap();
    tx.send(Some(TelemetrySample::new(payload))).unwrap();

    let call_req = test::TestRequest::post()
        .uri("/mcp")
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "get_indoor_climate" }
        }))
        .to_request();

    let call_res: serde_json::Value = test::call_and_read_body_json(&app, call_req).await;
    let text_content = call_res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text_content.contains("\"temp_int\":26"));
}

#[actix_rt::test]
async fn test_get_climate_trend_tool() {
    let (_tx, rx) = watch::channel(None);

    let history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(300))));

    let base = Instant::now();

    {
        let mut history_guard = history.write().await;

        history_guard.push(TelemetrySample::observed_at(
            TelemetryPayload::from_raw_dht11(24, 0, 50, 0),
            base - Duration::from_secs(30),
        ));

        history_guard.push(TelemetrySample::observed_at(
            TelemetryPayload::from_raw_dht11(26, 0, 60, 0),
            base,
        ));
    }

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx: rx,
                telemetry_history: history,
            }))
            .service(handlers::handle_mcp_rpc),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/mcp")
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "get_climate_trend",
                "arguments": {
                    "window_seconds": 60
                }
            }
        }))
        .to_request();

    let res: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    let text = res["result"]["content"][0]["text"].as_str().unwrap();

    let trend: weather_core::ClimateTrend = serde_json::from_str(text).unwrap();

    assert_eq!(trend.temperature.delta, 2.0);
    assert_eq!(trend.humidity.delta, 10.0);
    assert_eq!(
        trend.temperature.direction,
        weather_core::TrendDirection::Rising
    );
}

#[actix_rt::test]
async fn test_get_climate_trend_requires_enough_samples() {
    let (_tx, rx) = watch::channel(None);

    let history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(300))));

    {
        let mut history_guard = history.write().await;

        history_guard.push(TelemetrySample::new(TelemetryPayload::from_raw_dht11(
            24, 0, 50, 0,
        )));
    }

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx: rx,
                telemetry_history: history,
            }))
            .service(handlers::handle_mcp_rpc),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/mcp")
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "get_climate_trend",
                "arguments": {
                    "window_seconds": 60
                }
            }
        }))
        .to_request();

    let res: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    let text = res["result"]["content"][0]["text"].as_str().unwrap();

    assert!(text.contains("Not enough telemetry"));
}

#[actix_rt::test]
async fn test_get_climate_trend_rejects_zero_window() {
    let (_tx, rx) = watch::channel(None);

    let history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(300))));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx: rx,
                telemetry_history: history,
            }))
            .service(handlers::handle_mcp_rpc),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/mcp")
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "get_climate_trend",
                "arguments": {
                    "window_seconds": 0
                }
            }
        }))
        .to_request();

    let res: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    let text = res["result"]["content"][0]["text"].as_str().unwrap();

    assert!(text.contains("between 1 and 300"));
}

#[actix_rt::test]
async fn test_get_climate_trend_rejects_window_above_retention() {
    let (_tx, rx) = watch::channel(None);

    let history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(300))));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx: rx,
                telemetry_history: history,
            }))
            .service(handlers::handle_mcp_rpc),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/mcp")
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "get_climate_trend",
                "arguments": {
                    "window_seconds": 301
                }
            }
        }))
        .to_request();

    let res: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    let text = res["result"]["content"][0]["text"].as_str().unwrap();

    assert!(text.contains("between 1 and 300"));
}

#[actix_rt::test]
async fn test_get_climate_trend_requires_window_argument() {
    let (_tx, rx) = watch::channel(None);

    let history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(300))));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx: rx,
                telemetry_history: history,
            }))
            .service(handlers::handle_mcp_rpc),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/mcp")
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "get_climate_trend",
                "arguments": {}
            }
        }))
        .to_request();

    let res: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    let text = res["result"]["content"][0]["text"].as_str().unwrap();

    assert!(text.contains("window_seconds must be provided"));
}
