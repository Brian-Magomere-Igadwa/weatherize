use crate::state::TelemetrySample;
use std::time::Duration;
use tokio::sync::watch;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, LinesCodec};
use tracing::{error, info, warn};
use weather_core::TelemetryPayload;

pub async fn spawn_serial_ingestion_loop(
    port_path: String,
    baud_rate: u32,
    tx: watch::Sender<Option<TelemetrySample>>,
) {
    loop {
        info!(port = %port_path, baud = %baud_rate, "Attempting connection to serial port");

        match tokio_serial::new(&port_path, baud_rate).open_native_async() {
            Ok(serial_stream) => {
                info!("Successfully connected to serial device");
                process_serial_stream(serial_stream, &tx).await;
            }
            Err(err) => {
                warn!(error = %err, "Failed to open serial port. Retrying in 3s...");
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn process_serial_stream(stream: SerialStream, tx: &watch::Sender<Option<TelemetrySample>>) {
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
                        let _ = tx.send(Some(sample));
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
