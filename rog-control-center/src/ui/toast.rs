use std::time::Duration;
use zbus::DBusError;

/// Whether a toast reports a completed action or a failed one
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
}

impl ToastKind {
    /// Errors stay on screen longer so there is time to read the reason
    pub fn duration(self) -> Duration {
        match self {
            ToastKind::Success => Duration::from_secs(3),
            ToastKind::Error => Duration::from_secs(8),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastMessage {
    pub kind: ToastKind,
    pub text: String,
}

/// Build the toast for the result of a D-Bus call to asusd
pub fn toast_message(success: &str, fail: &str, result: &zbus::Result<()>) -> ToastMessage {
    match result {
        Ok(()) => ToastMessage {
            kind: ToastKind::Success,
            text: success.to_string(),
        },
        Err(e) => ToastMessage {
            kind: ToastKind::Error,
            text: format!("{fail}: {}", error_reason(e)),
        },
    }
}

/// The human-readable part of an error, without the D-Bus error name
fn error_reason(e: &zbus::Error) -> String {
    match e {
        zbus::Error::Failure(reason) => reason.clone(),
        zbus::Error::MethodError(_, Some(reason), _) => reason.clone(),
        zbus::Error::FDO(fdo) => fdo.description().unwrap_or_default().to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn success_result_gives_success_toast_with_success_text() {
        let toast = toast_message("Profile set", "Profile failed", &Ok(()));
        assert_eq!(toast.kind, ToastKind::Success);
        assert_eq!(toast.text, "Profile set");
    }

    #[test]
    fn error_result_gives_error_toast_with_reason() {
        let err = zbus::Error::Failure("value out of range".into());
        let toast = toast_message("Profile set", "Profile failed", &Err(err));
        assert_eq!(toast.kind, ToastKind::Error);
        assert_eq!(toast.text, "Profile failed: value out of range");
    }

    #[test]
    fn fdo_error_reason_omits_dbus_error_name() {
        let err = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied(
            "not in wheel group".into(),
        )));
        let toast = toast_message("Saved", "Save failed", &Err(err));
        assert_eq!(toast.text, "Save failed: not in wheel group");
    }

    #[test]
    fn errors_stay_visible_longer_than_successes() {
        assert_eq!(ToastKind::Success.duration(), Duration::from_secs(3));
        assert_eq!(ToastKind::Error.duration(), Duration::from_secs(8));
    }
}
