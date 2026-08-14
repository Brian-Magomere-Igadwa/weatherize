use crate::analysis::analyze_samples;
use crate::monitor::EnvironmentMonitor;

use crate::state::TelemetrySample;
use std::time::Duration;
use tokio::sync::{broadcast, watch, RwLock};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, LinesCodec};
use tracing::{error, info, warn};
use weather_core::{EnvironmentEvent, TelemetryPayload};

use crate::history::TelemetryHistory;
use std::sync::Arc;

pub async fn spawn_serial_ingestion_loop(
    port_path: String,
    baud_rate: u32,
    tx: watch::Sender<Option<TelemetrySample>>,
    history: Arc<RwLock<TelemetryHistory>>,
    event_tx: broadcast::Sender<EnvironmentEvent>,
) {
    let mut monitor = EnvironmentMonitor::new(Duration::from_secs(60));
    loop {
        info!(port = %port_path, baud = %baud_rate, "Attempting connection to serial port");

        match tokio_serial::new(&port_path, baud_rate).open_native_async() {
            Ok(serial_stream) => {
                info!("Successfully connected to serial device");
                process_serial_stream(serial_stream, &tx, &history, &mut monitor, &event_tx).await;
            }
            Err(err) => {
                warn!(error = %err, "Failed to open serial port. Retrying in 3s...");
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn process_serial_stream(
    stream: SerialStream,
    tx: &watch::Sender<Option<TelemetrySample>>,
    history: &Arc<RwLock<TelemetryHistory>>,
    monitor: &mut EnvironmentMonitor,
    event_tx: &broadcast::Sender<EnvironmentEvent>,
) {
    let mut lines = LinesCodec::new().framed(stream);

    while let Some(line_result) = lines.next().await {
        match line_result {
            Ok(line) => {
                if line.contains("FIRMWARE_INIT") {
                    info!(raw = %line, "Received board initialization signal");
                    continue;
                }

                match serde_json::from_str::<TelemetryPayload>(&line) {
                    Ok(payload) => {
                        info!(
                            temp = %payload.temperature_celsius(),
                            status = ?payload.status,
                            "Valid telemetry frame parsed"
                        );

                        let sample = TelemetrySample::new(payload);

                        // Preserve the latest snapshot first.
                        let _ = tx.send(Some(sample));

                        // Store the observation in rolling history.
                        history.write().await.push(sample);

                        // Analyze the most recent 30 seconds.
                        let recent_samples = {
                            let history = history.read().await;
                            history.recent(Duration::from_secs(30))
                        };

                        if let Ok(trend) = analyze_samples(&recent_samples) {
                            let events =
                                monitor.evaluate(&trend, sample.payload.status, sample.observed_at);

                            for event in events {
                                let _ = event_tx.send(event);
                            }
                        }
                    }
                    Err(err) => {
                        warn!(
                            error = %err,
                            raw = %line,
                            "Malformed JSON received over serial line"
                        );
                    }
                }
            }
            Err(err) => {
                error!(error = %err, "Serial stream connection interrupted");
                break;
            }
        }
    }
}
