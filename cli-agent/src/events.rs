#[derive(Debug, Clone)]
pub enum AgentEvent {
    UserMessage(String),
    Shutdown,
}
