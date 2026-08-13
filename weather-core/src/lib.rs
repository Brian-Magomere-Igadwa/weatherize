#![cfg_attr(not(feature = "std"), no_std)]
pub mod analysis;
pub mod domain;
pub mod protocol;

pub use analysis::{ClimateTrend, EnvironmentEvent, MetricTrend, TrendDirection};
pub use domain::SafetyStatus;
pub use protocol::TelemetryPayload;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_status_evaluation() {
        assert_eq!(SafetyStatus::evaluate(22, 45), SafetyStatus::Optimal);
        assert_eq!(SafetyStatus::evaluate(27, 50), SafetyStatus::Stuffy);
        assert_eq!(SafetyStatus::evaluate(24, 75), SafetyStatus::HighHumidity);
        assert_eq!(SafetyStatus::evaluate(33, 40), SafetyStatus::ExtremeHeat);
    }

    #[test]
    fn test_temperature_celsius_conversion() {
        let payload = TelemetryPayload::from_raw_dht11(24, 8, 65, 0);
        assert_eq!(payload.temperature_celsius(), 24.8);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_serde_json_roundtrip() {
        let payload = TelemetryPayload::from_raw_dht11(25, 4, 60, 0);
        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: TelemetryPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, deserialized);
    }
}
