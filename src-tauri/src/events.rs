pub enum AppEvent {
    DictationStarted,
    DictationStopped,
    TextRecognized {
        text: String,
        is_final: bool,
    },
    Error {
        message: String,
    },
}

impl AppEvent {
    pub fn name(&self) -> &str {
        match self {
            AppEvent::DictationStarted => "dictation-started",
            AppEvent::DictationStopped => "dictation-stopped",
            AppEvent::TextRecognized { .. } => "text-recognized",
            AppEvent::Error { .. } => "error",
        }
    }
}
