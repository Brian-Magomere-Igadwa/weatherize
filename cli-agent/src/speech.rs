pub struct SpeechEngine {
    enabled: bool,
}

impl SpeechEngine {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub async fn speak(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            return Ok(());
        }

        // Kokoro implementation comes in M6.2.
        println!("[Kūchō would speak]: {text}");

        Ok(())
    }
}
