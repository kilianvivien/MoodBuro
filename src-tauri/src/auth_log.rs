//! Development-only, fixed auth milestones. Never accepts URLs or credentials.
#[derive(Clone, Copy)]
pub enum Event {
    Started,
    CallbackVerified,
    CallbackRejected,
    TokenVerified,
    CredentialsSaved,
    SessionRestoring,
    SessionRestored,
    BrowserSessionFallback,
    RetryScheduled,
    RetryFinished,
    PrivateTokenPresent,
    PrivateTokenAbsent,
}
pub fn record(event: Event) {
    #[cfg(debug_assertions)]
    eprintln!(
        "[auth] {}",
        match event {
            Event::RetryScheduled => "waiting for autologin lockout before one restore retry",
            Event::RetryFinished => "autologin restore retry finished",
            Event::PrivateTokenPresent => "private token supplied at sign-in",
            Event::PrivateTokenAbsent => "private token absent at sign-in; browser session only",
            Event::Started => "sign-in window requested",
            Event::CallbackVerified => "one-use callback verified",
            Event::CallbackRejected => "callback rejected",
            Event::TokenVerified => "token verified with Moodle",
            Event::CredentialsSaved =>
                "credentials saved in OS credential store; metadata saved separately",
            Event::SessionRestoring => "restoring saved session",
            Event::SessionRestored => "one-use autologin navigation started",
            Event::BrowserSessionFallback =>
                "autologin unavailable; using retained browser session",
        }
    );
    #[cfg(not(debug_assertions))]
    let _ = event;
}
