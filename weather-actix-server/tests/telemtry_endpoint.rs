use actix_web::{test, web, App};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, RwLock};
use weather_actix_server::{handlers, history::TelemetryHistory, state::TelemetrySample};
use weather_core::TelemetryPayload;

use tokio::sync::broadcast;

#[actix_rt::test]
async fn test_telemetry_endpoint_flow() {
    let (tx, rx) = watch::channel(None);
    let history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(300))));

    let (environment_event_tx, _) = broadcast::channel(32);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx: rx,
                telemetry_history: history,
                environment_event_tx,
            }))
            .service(handlers::health_check)
            .service(handlers::get_current_telemetry),
    )
    .await;

    // 1. Initial State: Should return 503 Service Unavailable
    let req = test::TestRequest::get()
        .uri("/api/v1/telemetry")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 503);

    // 2. Broadcast Telemetry Frame
    let sample = TelemetryPayload::from_raw_dht11(23, 5, 55, 0);
    tx.send(Some(TelemetrySample::new(sample))).unwrap();

    // 3. Updated State: Should return 200 OK with payload
    let req = test::TestRequest::get()
        .uri("/api/v1/telemetry")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}
