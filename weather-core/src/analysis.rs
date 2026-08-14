#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum TrendDirection {
    Rising,
    Falling,
    Stable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MetricTrend {
    pub start: f32,
    pub current: f32,
    pub delta: f32,
    pub rate_per_minute: f32,
    pub direction: TrendDirection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ClimateTrend {
    pub temperature: MetricTrend,
    pub humidity: MetricTrend,
    pub sample_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trend_direction_values_are_distinct() {
        assert_ne!(TrendDirection::Rising, TrendDirection::Falling);
        assert_ne!(TrendDirection::Rising, TrendDirection::Stable);
        assert_ne!(TrendDirection::Falling, TrendDirection::Stable);
    }

    #[test]
    #[cfg(feature = "std")]
    fn trend_direction_serializes_as_expected() {
        let json = serde_json::to_string(&TrendDirection::Rising).unwrap();

        assert_eq!(json, "\"RISING\"");
    }

    #[test]
    fn climate_trend_can_represent_both_metrics() {
        let trend = ClimateTrend {
            temperature: MetricTrend {
                start: 24.0,
                current: 26.0,
                delta: 2.0,
                rate_per_minute: 4.0,
                direction: TrendDirection::Rising,
            },
            humidity: MetricTrend {
                start: 55.0,
                current: 60.0,
                delta: 5.0,
                rate_per_minute: 10.0,
                direction: TrendDirection::Rising,
            },
            sample_count: 4,
        };

        assert_eq!(trend.temperature.delta, 2.0);
        assert_eq!(trend.humidity.delta, 5.0);
        assert_eq!(trend.sample_count, 4);
    }
}
