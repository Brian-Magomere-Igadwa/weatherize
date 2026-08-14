use crate::state::TelemetrySample;
use weather_core::{ClimateTrend, MetricTrend, TrendDirection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendAnalysisError {
    InsufficientSamples,
    InvalidTimeRange,
}

pub fn analyze_samples(samples: &[TelemetrySample]) -> Result<ClimateTrend, TrendAnalysisError> {
    if samples.len() < 2 {
        return Err(TrendAnalysisError::InsufficientSamples);
    }

    let first = samples.first().unwrap();
    let last = samples.last().unwrap();

    let elapsed = last
        .observed_at
        .checked_duration_since(first.observed_at)
        .ok_or(TrendAnalysisError::InvalidTimeRange)?;

    let elapsed_minutes = elapsed.as_secs_f32() / 60.0;

    if elapsed_minutes <= 0.0 {
        return Err(TrendAnalysisError::InvalidTimeRange);
    }

    let start_temp = first.payload.temperature_celsius();
    let current_temp = last.payload.temperature_celsius();

    let start_humidity = humidity(&first.payload);
    let current_humidity = humidity(&last.payload);

    Ok(ClimateTrend {
        temperature: metric_trend(start_temp, current_temp, elapsed_minutes),
        humidity: metric_trend(start_humidity, current_humidity, elapsed_minutes),
        sample_count: samples.len(),
    })
}

fn metric_trend(start: f32, current: f32, elapsed_minutes: f32) -> MetricTrend {
    let delta = current - start;

    MetricTrend {
        start,
        current,
        delta,
        rate_per_minute: delta / elapsed_minutes,
        direction: direction(delta),
    }
}

fn direction(delta: f32) -> TrendDirection {
    if delta > 0.0 {
        TrendDirection::Rising
    } else if delta < 0.0 {
        TrendDirection::Falling
    } else {
        TrendDirection::Stable
    }
}

fn humidity(payload: &weather_core::TelemetryPayload) -> f32 {
    payload.humidity_int as f32 + (payload.humidity_dec as f32 / 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use weather_core::TelemetryPayload;

    fn sample_at(
        base: Instant,
        offset_secs: u64,
        temp_int: u8,
        temp_dec: u8,
        humidity_int: u8,
        humidity_dec: u8,
    ) -> TelemetrySample {
        TelemetrySample::observed_at(
            TelemetryPayload::from_raw_dht11(temp_int, temp_dec, humidity_int, humidity_dec),
            base + Duration::from_secs(offset_secs),
        )
    }
    #[test]
    fn calculates_rising_temperature_and_humidity() {
        let base = Instant::now();

        let samples = [
            sample_at(base, 0, 24, 0, 50, 0),
            sample_at(base, 30, 26, 0, 55, 0),
        ];

        let trend = analyze_samples(&samples).unwrap();

        assert_eq!(trend.temperature.start, 24.0);
        assert_eq!(trend.temperature.current, 26.0);
        assert_eq!(trend.temperature.delta, 2.0);
        assert_eq!(trend.temperature.rate_per_minute, 4.0);
        assert_eq!(trend.temperature.direction, TrendDirection::Rising);

        assert_eq!(trend.humidity.delta, 5.0);
        assert_eq!(trend.humidity.rate_per_minute, 10.0);
        assert_eq!(trend.humidity.direction, TrendDirection::Rising);

        assert_eq!(trend.sample_count, 2);
    }
    #[test]
    fn calculates_falling_trends() {
        let base = Instant::now();

        let samples = [
            sample_at(base, 0, 28, 0, 65, 0),
            sample_at(base, 60, 26, 0, 55, 0),
        ];

        let trend = analyze_samples(&samples).unwrap();

        assert_eq!(trend.temperature.delta, -2.0);
        assert_eq!(trend.temperature.rate_per_minute, -2.0);
        assert_eq!(trend.temperature.direction, TrendDirection::Falling);

        assert_eq!(trend.humidity.delta, -10.0);
        assert_eq!(trend.humidity.direction, TrendDirection::Falling);
    }
    #[test]
    fn detects_stable_metrics() {
        let base = Instant::now();

        let samples = [
            sample_at(base, 0, 24, 0, 50, 0),
            sample_at(base, 60, 24, 0, 50, 0),
        ];

        let trend = analyze_samples(&samples).unwrap();

        assert_eq!(trend.temperature.delta, 0.0);
        assert_eq!(trend.temperature.rate_per_minute, 0.0);
        assert_eq!(trend.temperature.direction, TrendDirection::Stable);

        assert_eq!(trend.humidity.direction, TrendDirection::Stable);
    }

    #[test]
    fn rejects_insufficient_samples() {
        let base = Instant::now();

        let samples = [sample_at(base, 0, 24, 0, 50, 0)];

        assert_eq!(
            analyze_samples(&samples),
            Err(TrendAnalysisError::InsufficientSamples)
        );
    }

    #[test]
    fn rejects_zero_duration_window() {
        let base = Instant::now();

        let samples = [
            sample_at(base, 0, 24, 0, 50, 0),
            sample_at(base, 0, 25, 0, 55, 0),
        ];

        assert_eq!(
            analyze_samples(&samples),
            Err(TrendAnalysisError::InvalidTimeRange)
        );
    }
}
