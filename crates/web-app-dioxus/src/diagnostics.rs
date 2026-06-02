//! Logging for failures that are surfaced in a component rather than a notification.
//!
//! Notifications log themselves when pushed. A failure shown only in a component would otherwise
//! leave no trace in the log, so [`log_failure`] records it here, choosing the level from whether
//! the error is recoverable.

use std::fmt::Display;

use valens_domain::Recoverable;

/// Log a failed operation. Recoverable (transient or expected) conditions are logged at debug,
/// genuine faults at error. `action` is an infinitive phrase such as `"load body weight"` or
/// `"sign in"`.
pub fn log_failure(action: &str, err: &(impl Display + Recoverable)) {
    if err.recoverable() {
        log::debug!("failed to {action}: {err}");
    } else {
        log::error!("failed to {action}: {err}");
    }
}
