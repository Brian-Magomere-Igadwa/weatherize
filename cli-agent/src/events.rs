use weather_core::EnvironmentEvent;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    UserMessage(String),
    Environment(EnvironmentEvent),
    Shutdown,
}
