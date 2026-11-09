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
}
pub fn record(event: Event) {
    #[cfg(debug_assertions)]
    eprintln!(
        "[auth] {}",
        match event {
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
