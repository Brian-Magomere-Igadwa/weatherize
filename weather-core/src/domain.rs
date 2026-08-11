// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
// #[repr(u8)]
// pub enum SafetyStatus {
//     Optimal = 0,
//     Stuffy = 1,
//     HighHumidity = 2,
//     ExtremeHeat = 3,
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[repr(u8)]
pub enum SafetyStatus {
    Optimal = 0,
    Stuffy = 1,
    HighHumidity = 2,
    ExtremeHeat = 3,
}

impl SafetyStatus {
    pub fn evaluate(temp_int: u8, humidity_int: u8) -> Self {
        match (temp_int, humidity_int) {
            (t, _) if t >= 32 => SafetyStatus::ExtremeHeat,
            (_, h) if h >= 70 => SafetyStatus::HighHumidity,
            (t, h) if t >= 26 || h >= 60 => SafetyStatus::Stuffy,
            _ => SafetyStatus::Optimal,
        }
    }
}

impl ufmt::uDisplay for SafetyStatus {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        match self {
            SafetyStatus::Optimal => f.write_str("OPTIMAL"),
            SafetyStatus::Stuffy => f.write_str("STUFFY"),
            SafetyStatus::HighHumidity => f.write_str("HIGH_HUMIDITY"),
            SafetyStatus::ExtremeHeat => f.write_str("EXTREME_HEAT"),
        }
    }
}
