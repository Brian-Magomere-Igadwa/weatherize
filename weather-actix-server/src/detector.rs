use weather_core::{ClimateTrend, EnvironmentEvent, SafetyStatus};

const RAPID_TEMPERATURE_DELTA_C: f32 = 1.5;
const RAPID_HUMIDITY_DELTA_PERCENT: f32 = 10.0;

pub fn detect_environment_events(
    trend: &ClimateTrend,
    previous_status: SafetyStatus,
    current_status: SafetyStatus,
) -> Vec<EnvironmentEvent> {
    let mut events = Vec::new();

    if trend.temperature.delta.abs() >= RAPID_TEMPERATURE_DELTA_C {
        events.push(EnvironmentEvent::TemperatureChangedRapidly {
            delta_celsius: trend.temperature.delta,
            rate_per_minute: trend.temperature.rate_per_minute,
        });
    }

    if trend.humidity.delta.abs() >= RAPID_HUMIDITY_DELTA_PERCENT {
        events.push(EnvironmentEvent::HumidityChangedRapidly {
            delta_percent: trend.humidity.delta,
            rate_per_minute: trend.humidity.rate_per_minute,
        });
    }

    if previous_status != current_status {
        events.push(EnvironmentEvent::SafetyStatusChanged {
            from: previous_status,
            to: current_status,
        });
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use weather_core::{MetricTrend, TrendDirection};

    fn stable_trend() -> ClimateTrend {
        ClimateTrend {
            temperature: MetricTrend {
                start: 24.0,
                current: 24.0,
                delta: 0.0,
                rate_per_minute: 0.0,
                direction: TrendDirection::Stable,
            },
            humidity: MetricTrend {
                start: 50.0,
                current: 50.0,
                delta: 0.0,
                rate_per_minute: 0.0,
                direction: TrendDirection::Stable,
            },
            sample_count: 2,
        }
    }

    #[test]
    fn detects_rapid_temperature_change() {
        let trend = ClimateTrend {
            temperature: MetricTrend {
                start: 24.0,
                current: 26.0,
                delta: 2.0,
                rate_per_minute: 4.0,
                direction: TrendDirection::Rising,
            },
            humidity: MetricTrend {
                start: 50.0,
                current: 52.0,
                delta: 2.0,
                rate_per_minute: 4.0,
                direction: TrendDirection::Rising,
            },
            sample_count: 2,
        };

        let events =
            detect_environment_events(&trend, SafetyStatus::Optimal, SafetyStatus::Optimal);

        assert_eq!(events.len(), 1);

        assert!(matches!(
            events[0],
            EnvironmentEvent::TemperatureChangedRapidly { .. }
        ));
    }

    #[test]
    fn detects_rapid_humidity_change() {
        let trend = ClimateTrend {
            temperature: MetricTrend {
                start: 24.0,
                current: 24.3,
                delta: 0.3,
                rate_per_minute: 0.6,
                direction: TrendDirection::Rising,
            },
            humidity: MetricTrend {
                start: 50.0,
                current: 62.0,
                delta: 12.0,
                rate_per_minute: 24.0,
                direction: TrendDirection::Rising,
            },
            sample_count: 2,
        };

        let events =
            detect_environment_events(&trend, SafetyStatus::Optimal, SafetyStatus::Optimal);

        assert_eq!(events.len(), 1);

        assert!(matches!(
            events[0],
            EnvironmentEvent::HumidityChangedRapidly { .. }
        ));
    }

    #[test]
    fn detects_safety_status_change() {
        let trend = stable_trend();

        let events = detect_environment_events(&trend, SafetyStatus::Optimal, SafetyStatus::Stuffy);

        assert_eq!(
            events,
            vec![EnvironmentEvent::SafetyStatusChanged {
                from: SafetyStatus::Optimal,
                to: SafetyStatus::Stuffy,
            }]
        );
    }

    #[test]
    fn ignores_small_changes() {
        let trend = ClimateTrend {
            temperature: MetricTrend {
                start: 24.0,
                current: 24.4,
                delta: 0.4,
                rate_per_minute: 0.8,
                direction: TrendDirection::Rising,
            },
            humidity: MetricTrend {
                start: 50.0,
                current: 53.0,
                delta: 3.0,
                rate_per_minute: 6.0,
                direction: TrendDirection::Rising,
            },
            sample_count: 2,
        };

        let events =
            detect_environment_events(&trend, SafetyStatus::Optimal, SafetyStatus::Optimal);

        assert!(events.is_empty());
    }

    #[test]
    fn detects_rapid_downward_changes() {
        let trend = ClimateTrend {
            temperature: MetricTrend {
                start: 28.0,
                current: 26.0,
                delta: -2.0,
                rate_per_minute: -4.0,
                direction: TrendDirection::Falling,
            },
            humidity: MetricTrend {
                start: 70.0,
                current: 58.0,
                delta: -12.0,
                rate_per_minute: -24.0,
                direction: TrendDirection::Falling,
            },
            sample_count: 2,
        };

        let events =
            detect_environment_events(&trend, SafetyStatus::Optimal, SafetyStatus::Optimal);

        assert_eq!(events.len(), 2);

        assert!(matches!(
            events[0],
            EnvironmentEvent::TemperatureChangedRapidly {
                delta_celsius: -2.0,
                ..
            }
        ));

        assert!(matches!(
            events[1],
            EnvironmentEvent::HumidityChangedRapidly {
                delta_percent: -12.0,
                ..
            }
        ));
    }

    #[test]
    fn detects_changes_at_exact_threshold() {
        let trend = ClimateTrend {
            temperature: MetricTrend {
                start: 24.0,
                current: 25.5,
                delta: 1.5,
                rate_per_minute: 3.0,
                direction: TrendDirection::Rising,
            },
            humidity: MetricTrend {
                start: 50.0,
                current: 60.0,
                delta: 10.0,
                rate_per_minute: 20.0,
                direction: TrendDirection::Rising,
            },
            sample_count: 2,
        };

        let events =
            detect_environment_events(&trend, SafetyStatus::Optimal, SafetyStatus::Optimal);

        assert_eq!(events.len(), 2);

        assert!(matches!(
            events[0],
            EnvironmentEvent::TemperatureChangedRapidly {
                delta_celsius: 1.5,
                ..
            }
        ));

        assert!(matches!(
            events[1],
            EnvironmentEvent::HumidityChangedRapidly {
                delta_percent: 10.0,
                ..
            }
        ));
    }
}
