use crate::state::TelemetrySample;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct TelemetryHistory {
    samples: VecDeque<TelemetrySample>,
    retention: Duration,
}

impl TelemetryHistory {
    pub fn new(retention: Duration) -> Self {
        Self {
            samples: VecDeque::new(),
            retention,
        }
    }

    pub fn push(&mut self, sample: TelemetrySample) {
        self.samples.push_back(sample);
        self.prune();
    }

    pub fn recent(&self, window: Duration) -> Vec<TelemetrySample> {
        let cutoff = Instant::now() - window;

        self.samples
            .iter()
            .copied()
            .filter(|sample| sample.observed_at >= cutoff)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    fn prune(&mut self) {
        let cutoff = Instant::now() - self.retention;

        while self
            .samples
            .front()
            .is_some_and(|sample| sample.observed_at < cutoff)
        {
            self.samples.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weather_core::TelemetryPayload;

    #[test]
    fn stores_telemetry_samples() {
        let mut history = TelemetryHistory::new(Duration::from_secs(300));

        let payload = TelemetryPayload::from_raw_dht11(24, 5, 55, 0);
        history.push(TelemetrySample::new(payload));

        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn recent_returns_samples_within_window() {
        let mut history = TelemetryHistory::new(Duration::from_secs(300));

        history.push(TelemetrySample::new(TelemetryPayload::from_raw_dht11(
            24, 0, 55, 0,
        )));

        history.push(TelemetrySample::new(TelemetryPayload::from_raw_dht11(
            25, 0, 56, 0,
        )));

        let samples = history.recent(Duration::from_secs(30));

        assert_eq!(samples.len(), 2);
    }

    #[test]
    fn prunes_samples_older_than_retention() {
        let mut history = TelemetryHistory::new(Duration::from_secs(300));

        let old_payload = TelemetryPayload::from_raw_dht11(20, 0, 40, 0);

        let current_payload = TelemetryPayload::from_raw_dht11(25, 0, 60, 0);

        history.push(TelemetrySample::observed_at(
            old_payload,
            Instant::now() - Duration::from_secs(301),
        ));

        history.push(TelemetrySample::new(current_payload));

        assert_eq!(history.len(), 1);
    }
}
