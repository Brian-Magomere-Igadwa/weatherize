use std::time::Instant;
use weather_core::TelemetryPayload;

#[derive(Debug, Clone, Copy)]
pub struct TelemetrySample {
    pub observed_at: Instant,
    pub payload: TelemetryPayload,
}

impl TelemetrySample {
    pub fn new(payload: TelemetryPayload) -> Self {
        Self::observed_at(payload, Instant::now())
    }

    pub fn observed_at(payload: TelemetryPayload, observed_at: Instant) -> Self {
        Self {
            observed_at,
            payload,
        }
    }
}
