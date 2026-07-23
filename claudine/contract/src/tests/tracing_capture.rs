use std::sync::{Arc, Mutex};
use tracing::{Event, Subscriber, field::Visit};
use tracing_subscriber::layer::{Context, Layer};

#[derive(Clone)]
pub struct Capture {
    events: Arc<Mutex<Vec<String>>>,
}

impl Capture {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    pub fn contains(&self, needle: &str) -> bool {
        self.events().iter().any(|event| event.contains(needle))
    }
}

impl<S> Layer<S> for Capture
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = Visitor::default();
        event.record(&mut visitor);
        let mut parts = Vec::new();
        if let Some(message) = visitor.message {
            parts.push(message);
        }
        parts.extend(visitor.fields);
        self.events.lock().unwrap().push(parts.join(" "));
    }
}

#[derive(Default)]
struct Visitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        } else {
            self.fields.push(format!("{}={:?}", field.name(), value));
        }
    }
}
