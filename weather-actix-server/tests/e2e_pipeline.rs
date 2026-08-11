use actix_web::{test, web, App};
use tokio::sync::watch;
use weather_actix_server::{handlers, mcp};
use weather_core::{SafetyStatus, TelemetryPayload};

#[actix_rt::test]
async fn test_full_system_e2e_flow() {
    let (tx, rx) = watch::channel(None);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState { telemetry_rx: rx }))
            .service(handlers::health_check)
            .service(handlers::get_current_telemetry)
            .service(handlers::handle_mcp_rpc),
    )
    .await;

    // 1. Verify Health Endpoint
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // 2. Simulate Hardware DHT11 Ingestion Loop
    let sample = TelemetryPayload::from_raw_dht11(28, 4, 68, 0);
    tx.send(Some(sample)).unwrap();

    // 3. Verify REST API Output matches ingested data
    let req = test::TestRequest::get()
        .uri("/api/v1/telemetry")
        .to_request();
    let payload: TelemetryPayload = test::call_and_read_body_json(&app, req).await;
    assert_eq!(payload.temp_int, 28);
    assert_eq!(payload.status, SafetyStatus::Stuffy);

    // 4. Verify MCP RPC Tool Execution reads the snapshot
    let mcp_req = test::TestRequest::post()
        .uri("/mcp")
        .set_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "tools/call",
            "params": { "name": "get_indoor_climate" }
        }))
        .to_request();

    let mcp_res: serde_json::Value = test::call_and_read_body_json(&app, mcp_req).await;
    let response_text = mcp_res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(response_text.contains("\"humidity_int\":68"));
}
