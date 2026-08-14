use actix_web::{test, web, App};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch, RwLock};

use weather_actix_server::{handlers, history::TelemetryHistory};

#[actix_rt::test]
async fn test_environment_events_endpoint_opens_sse_stream() {
    let (_telemetry_tx, telemetry_rx) = watch::channel(None);

    let history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(300))));

    let (environment_event_tx, _) = broadcast::channel(32);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx,
                telemetry_history: history,
                environment_event_tx,
            }))
            .service(handlers::stream_environment_events),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/events").to_request();

    let response = test::call_service(&app, req).await;

    assert!(response.status().is_success());

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );

    assert_eq!(response.headers().get("cache-control").unwrap(), "no-cache");
}
