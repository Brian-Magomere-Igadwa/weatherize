use std::time::{Duration, Instant};
use weather_core::{ClimateTrend, EnvironmentEvent, SafetyStatus};

use crate::detector::detect_environment_events;

const STUFFY_TEMP_EXIT_C: f32 = 25.0;
const STUFFY_HUMIDITY_EXIT_PERCENT: f32 = 57.0;

pub struct EnvironmentMonitor {
    previous_status: Option<SafetyStatus>,
    last_temperature_event_at: Option<Instant>,
    last_humidity_event_at: Option<Instant>,
    last_status_event_at: Option<Instant>,
    cooldown: Duration,
}

impl EnvironmentMonitor {
    pub fn new(cooldown: Duration) -> Self {
        Self {
            previous_status: None,
            last_temperature_event_at: None,
            last_humidity_event_at: None,
            last_status_event_at: None,
            cooldown,
        }
    }

    pub fn evaluate(
        &mut self,
        trend: &ClimateTrend,
        current_status: SafetyStatus,
        now: Instant,
    ) -> Vec<EnvironmentEvent> {
        let previous_status = match self.previous_status {
            Some(status) => status,
            None => {
                self.previous_status = Some(current_status);
                return Vec::new();
            }
        };

        let effective_status = self.apply_status_hysteresis(previous_status, current_status, trend);

        let candidates = detect_environment_events(trend, previous_status, effective_status);

        let mut emitted = Vec::new();

        for event in candidates {
            let allowed = match event {
                EnvironmentEvent::TemperatureChangedRapidly { .. } => {
                    cooldown_elapsed(self.last_temperature_event_at, now, self.cooldown)
                }

                EnvironmentEvent::HumidityChangedRapidly { .. } => {
                    cooldown_elapsed(self.last_humidity_event_at, now, self.cooldown)
                }

                EnvironmentEvent::SafetyStatusChanged { .. } => {
                    cooldown_elapsed(self.last_status_event_at, now, self.cooldown)
                }
            };

            if allowed {
                match event {
                    EnvironmentEvent::TemperatureChangedRapidly { .. } => {
                        self.last_temperature_event_at = Some(now);
                    }

                    EnvironmentEvent::HumidityChangedRapidly { .. } => {
                        self.last_humidity_event_at = Some(now);
                    }

                    EnvironmentEvent::SafetyStatusChanged { .. } => {
                        self.last_status_event_at = Some(now);
                    }
                }

                emitted.push(event);
            }
        }

        self.previous_status = Some(effective_status);

        emitted
    }

    fn apply_status_hysteresis(
        &self,
        previous_status: SafetyStatus,
        observed_status: SafetyStatus,
        trend: &ClimateTrend,
    ) -> SafetyStatus {
        if previous_status == SafetyStatus::Stuffy && observed_status == SafetyStatus::Optimal {
            let temp_recovered = trend.temperature.current <= STUFFY_TEMP_EXIT_C;

            let humidity_recovered = trend.humidity.current <= STUFFY_HUMIDITY_EXIT_PERCENT;

            if !temp_recovered || !humidity_recovered {
                return SafetyStatus::Stuffy;
            }
        }

        observed_status
    }
}

fn cooldown_elapsed(previous: Option<Instant>, now: Instant, cooldown: Duration) -> bool {
    match previous {
        None => true,
        Some(last) => now
            .checked_duration_since(last)
            .is_some_and(|elapsed| elapsed >= cooldown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weather_core::{MetricTrend, TrendDirection};

    fn rapid_temperature_trend() -> ClimateTrend {
        ClimateTrend {
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
        }
    }

    fn rapid_humidity_trend() -> ClimateTrend {
        ClimateTrend {
            temperature: MetricTrend {
                start: 24.0,
                current: 24.2,
                delta: 0.2,
                rate_per_minute: 0.4,
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
        }
    }

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
    fn first_observation_only_initializes_state() {
        let mut monitor = EnvironmentMonitor::new(Duration::from_secs(60));
        let now = Instant::now();

        let events = monitor.evaluate(&rapid_temperature_trend(), SafetyStatus::Optimal, now);

        assert!(events.is_empty());
    }

    #[test]
    fn emits_temperature_event_after_initialization() {
        let mut monitor = EnvironmentMonitor::new(Duration::from_secs(60));
        let start = Instant::now();

        monitor.evaluate(&stable_trend(), SafetyStatus::Optimal, start);

        let events = monitor.evaluate(
            &rapid_temperature_trend(),
            SafetyStatus::Optimal,
            start + Duration::from_secs(1),
        );

        assert_eq!(events.len(), 1);

        assert!(matches!(
            events[0],
            EnvironmentEvent::TemperatureChangedRapidly { .. }
        ));
    }

    #[test]
    fn suppresses_repeated_temperature_event_during_cooldown() {
        let mut monitor = EnvironmentMonitor::new(Duration::from_secs(60));
        let start = Instant::now();

        monitor.evaluate(&stable_trend(), SafetyStatus::Optimal, start);

        let first = monitor.evaluate(
            &rapid_temperature_trend(),
            SafetyStatus::Optimal,
            start + Duration::from_secs(1),
        );

        let second = monitor.evaluate(
            &rapid_temperature_trend(),
            SafetyStatus::Optimal,
            start + Duration::from_secs(30),
        );

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    #[test]
    fn emits_temperature_event_again_after_cooldown() {
        let mut monitor = EnvironmentMonitor::new(Duration::from_secs(60));
        let start = Instant::now();

        monitor.evaluate(&stable_trend(), SafetyStatus::Optimal, start);

        let first = monitor.evaluate(
            &rapid_temperature_trend(),
            SafetyStatus::Optimal,
            start + Duration::from_secs(1),
        );

        let second = monitor.evaluate(
            &rapid_temperature_trend(),
            SafetyStatus::Optimal,
            start + Duration::from_secs(61),
        );

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
    }

    #[test]
    fn temperature_and_humidity_have_independent_cooldowns() {
        let mut monitor = EnvironmentMonitor::new(Duration::from_secs(60));
        let start = Instant::now();

        monitor.evaluate(&stable_trend(), SafetyStatus::Optimal, start);

        let temp_events = monitor.evaluate(
            &rapid_temperature_trend(),
            SafetyStatus::Optimal,
            start + Duration::from_secs(1),
        );

        let humidity_events = monitor.evaluate(
            &rapid_humidity_trend(),
            SafetyStatus::Optimal,
            start + Duration::from_secs(10),
        );

        assert_eq!(temp_events.len(), 1);
        assert_eq!(humidity_events.len(), 1);

        assert!(matches!(
            humidity_events[0],
            EnvironmentEvent::HumidityChangedRapidly { .. }
        ));
    }

    #[test]
    fn suppresses_stuffy_recovery_near_threshold() {
        let mut monitor = EnvironmentMonitor::new(Duration::from_secs(60));
        let start = Instant::now();

        monitor.evaluate(&stable_trend(), SafetyStatus::Stuffy, start);

        let near_threshold = ClimateTrend {
            temperature: MetricTrend {
                start: 26.0,
                current: 25.5,
                delta: -0.5,
                rate_per_minute: -1.0,
                direction: TrendDirection::Falling,
            },
            humidity: MetricTrend {
                start: 60.0,
                current: 59.0,
                delta: -1.0,
                rate_per_minute: -2.0,
                direction: TrendDirection::Falling,
            },
            sample_count: 2,
        };

        let events = monitor.evaluate(
            &near_threshold,
            SafetyStatus::Optimal,
            start + Duration::from_secs(10),
        );

        assert!(events.is_empty());
    }

    #[test]
    fn emits_recovery_after_hysteresis_thresholds_are_cleared() {
        let mut monitor = EnvironmentMonitor::new(Duration::from_secs(60));
        let start = Instant::now();

        monitor.evaluate(&stable_trend(), SafetyStatus::Stuffy, start);

        let recovered = ClimateTrend {
            temperature: MetricTrend {
                start: 26.0,
                current: 24.5,
                delta: -1.5,
                rate_per_minute: -3.0,
                direction: TrendDirection::Falling,
            },
            humidity: MetricTrend {
                start: 60.0,
                current: 56.0,
                delta: -4.0,
                rate_per_minute: -8.0,
                direction: TrendDirection::Falling,
            },
            sample_count: 2,
        };

        let events = monitor.evaluate(
            &recovered,
            SafetyStatus::Optimal,
            start + Duration::from_secs(10),
        );

        assert!(events.iter().any(|event| matches!(
            event,
            EnvironmentEvent::SafetyStatusChanged {
                from: SafetyStatus::Stuffy,
                to: SafetyStatus::Optimal,
            }
        )));
    }
}
