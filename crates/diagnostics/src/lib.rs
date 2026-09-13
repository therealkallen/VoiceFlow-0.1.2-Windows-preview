use shared_protocol::DiagnosticEvent;

#[derive(Debug, Default)]
pub struct DiagnosticsStore {
    events: Vec<DiagnosticEvent>,
}

impl DiagnosticsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, event: DiagnosticEvent) {
        self.events.push(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn snapshot(&self) -> &[DiagnosticEvent] {
        &self.events
    }
}
