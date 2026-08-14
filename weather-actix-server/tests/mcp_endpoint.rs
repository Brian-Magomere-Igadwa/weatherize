use actix_web::{test, web, App};
use tokio::sync::watch;
use weather_actix_server::{handlers, state::TelemetrySample};
use weather_core::TelemetryPayload;

#[actix_rt::test]
async fn test_mcp_tools_list_and_call() {
    let (tx, rx) = watch::channel(None);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState { telemetry_rx: rx }))
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
