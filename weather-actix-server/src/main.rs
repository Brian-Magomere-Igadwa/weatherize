use actix_web::{web, App, HttpServer};
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;
use weather_actix_server::{handlers, serial};

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, RwLock};
use weather_actix_server::history::TelemetryHistory;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let serial_port = env::var("SERIAL_PORT").unwrap_or_else(|_| "/dev/ttyACM0".to_string());
    let baud_rate: u32 = env::var("BAUD_RATE")
        .unwrap_or_else(|_| "57600".to_string())
        .parse()
        .expect("BAUD_RATE must be a valid u32");
    let server_host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid u16");

    let (tx, rx) = watch::channel(None);

    let telemetry_history = Arc::new(RwLock::new(TelemetryHistory::new(Duration::from_secs(
        5 * 60,
    ))));

    tokio::spawn(serial::spawn_serial_ingestion_loop(
        serial_port,
        baud_rate,
        tx,
        telemetry_history.clone(),
    ));

    info!(address = %format!("{}:{}", server_host, server_port), "Starting Actix web server");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(handlers::AppState {
                telemetry_rx: rx.clone(),
                telemetry_history: telemetry_history.clone(),
            }))
            .service(handlers::health_check)
            .service(handlers::get_current_telemetry)
            .service(handlers::handle_mcp_rpc)
    })
    .bind((server_host.as_str(), server_port))?
    .run()
    .await
}
