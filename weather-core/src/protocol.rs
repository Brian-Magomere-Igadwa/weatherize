use crate::domain::SafetyStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TelemetryPayload {
    pub temp_int: u8,
    pub temp_dec: u8,
    pub humidity_int: u8,
    pub humidity_dec: u8,
    pub status: SafetyStatus,
}

impl TelemetryPayload {
    pub fn from_raw_dht11(temp_int: u8, temp_dec: u8, humidity_int: u8, humidity_dec: u8) -> Self {
        let status = SafetyStatus::evaluate(temp_int, humidity_int);
        Self {
            temp_int,
            temp_dec,
            humidity_int,
            humidity_dec,
            status,
        }
    }

    pub fn temperature_celsius(&self) -> f32 {
        self.temp_int as f32 + (self.temp_dec as f32 / 10.0)
    }
}

// impl ufmt::uDisplay for TelemetryPayload {
//     fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
//     where
//         W: ufmt::uWrite + ?Sized,
//     {
//         f.write_str("{\"temp\":")?;
//         ufmt::uwrite!(f, "{}.{}", self.temp_int, self.temp_dec)?;
//         f.write_str(",\"humidity\":")?;
//         ufmt::uwrite!(f, "{}.{}", self.humidity_int, self.humidity_dec)?;
//         f.write_str(",\"status\":\"")?;
//         ufmt::uwrite!(f, "{}", self.status)?;
//         f.write_str("\"}")
//     }
// }

impl ufmt::uDisplay for TelemetryPayload {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        f.write_str("{\"temp_int\":")?;
        ufmt::uwrite!(f, "{}", self.temp_int)?;
        f.write_str(",\"temp_dec\":")?;
        ufmt::uwrite!(f, "{}", self.temp_dec)?;
        f.write_str(",\"humidity_int\":")?;
        ufmt::uwrite!(f, "{}", self.humidity_int)?;
        f.write_str(",\"humidity_dec\":")?;
        ufmt::uwrite!(f, "{}", self.humidity_dec)?;
        f.write_str(",\"status\":\"")?;
        ufmt::uwrite!(f, "{}", self.status)?;
        f.write_str("\"}")
    }
}
