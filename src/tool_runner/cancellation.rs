use super::*;

/// Cancellation shared with the caller. Cancelling drops active HTTP/callback
/// futures and prevents subsequent work; already completed side effects remain.
#[derive(Clone)]
pub struct ToolRunnerCancellation {
    signal: watch::Sender<bool>,
}
impl Default for ToolRunnerCancellation {
    fn default() -> Self {
        let (signal, _) = watch::channel(false);
        Self { signal }
    }
}
impl ToolRunnerCancellation {
    /// Create an uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }
    /// Stop active and future work for runs using this token.
    pub fn cancel(&self) {
        self.signal.send_replace(true);
    }
    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        *self.signal.borrow()
    }
    pub(super) async fn cancelled(&self) {
        let mut receiver = self.signal.subscribe();
        if *receiver.borrow() {
            return;
        }
        while receiver.changed().await.is_ok() {
            if *receiver.borrow() {
                return;
            }
        }
    }
}
