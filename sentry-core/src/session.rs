//! Release Health Sessions
//!
//! https://develop.sentry.dev/sdk/sessions/

use std::fmt;
use std::time::Instant;

use crate::protocol::{Event, Level};
use crate::types::{DateTime, Utc, Uuid};

/// Represents the status of a session.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SessionStatus {
    Ok,
    Crashed,
    Abnormal,
    Exited,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SessionStatus::Ok => write!(f, "ok"),
            SessionStatus::Crashed => write!(f, "crashed"),
            SessionStatus::Abnormal => write!(f, "abnormal"),
            SessionStatus::Exited => write!(f, "exited"),
        }
    }
}

impl serde::Serialize for SessionStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub enum SessionUpdate {
    NeedsFlushing(Session),
    Unchanged,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    session_id: Uuid,
    status: SessionStatus,
    errors: usize,
    started: Instant,
    started_utc: DateTime<Utc>,
    duration: Option<f64>,
    init: bool,
    dirty: bool,
}

impl Session {
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            status: SessionStatus::Ok,
            errors: 0,
            started: Instant::now(),
            started_utc: Utc::now(),
            duration: None,
            init: true,
            dirty: true,
        }
    }

    pub(crate) fn set_user(&mut self, user: ()) {}

    pub(crate) fn update_from_event(&mut self, event: &Event<'static>) -> SessionUpdate {
        let mut has_error = event.level >= Level::Error;
        let mut is_crash = false;
        for exc in &event.exception.values {
            has_error = true;
            if let Some(mechanism) = &exc.mechanism {
                if matches!(mechanism.handled, Some(false)) {
                    is_crash = true;
                    break;
                }
            }
        }

        if is_crash {
            self.status = SessionStatus::Crashed;
        }
        if has_error {
            self.errors += 1;
            self.dirty = true;
        }

        if self.dirty {
            self.dirty = false;
            let session = self.clone();
            self.init = false;
            SessionUpdate::NeedsFlushing(session)
        } else {
            SessionUpdate::Unchanged
        }
    }

    pub(crate) fn close(&mut self) {
        self.duration = Some(self.started.elapsed().as_secs_f64());
        if self.status == SessionStatus::Ok {
            self.status = SessionStatus::Exited;
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Session::new()
    }
}

impl serde::Serialize for Session {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        //
        let mut session = serializer.serialize_struct("Session", 6)?;
        session.serialize_field("sid", &self.session_id)?;
        session.serialize_field(
            "status",
            match self.status {
                SessionStatus::Ok => "ok",
                SessionStatus::Crashed => "crashed",
                SessionStatus::Abnormal => "abnormal",
                SessionStatus::Exited => "exited",
            },
        )?;
        session.serialize_field("errors", &self.errors)?;
        session.serialize_field("started", &self.started_utc)?;
        if let Some(duration) = self.duration {
            session.serialize_field("duration", &duration)?;
        } else {
            session.skip_field("duration")?;
        }
        if self.init {
            session.serialize_field("init", &true)?;
        } else {
            session.skip_field("init")?;
        }
        session.end()
        //serializer.serialize_str(&self.to_string())
    }
}

#[cfg(all(test, feature = "test"))]
mod tests {
    use crate as sentry;
    use crate::test::with_captured_envelopes_options;
    use crate::ClientOptions;

    /// let mut body = Vec::new();
    /// // the second envelope contains the session
    /// envelopes[1].to_writer(&mut body).unwrap();
    /// assert!(&body.starts_with(b"{}\n{\"type\":\"session\","));
    /// let json: serde_json::Value = serde_json::from_slice(body.split(|c| *c == b'\n').nth(2).unwrap()).unwrap();
    /// let sess = json.as_object().unwrap();
    /// assert_eq!(sess["status"].as_str().unwrap(), "exited");
    /// assert_eq!(sess["errors"].as_u64().unwrap(), 1);
    /// assert_eq!(sess["init"].as_bool(), Some(true));
    /// assert!(sess["duration"].as_f64().unwrap() > 0.01);
    /// //assert_eq!(std::str::from_utf8(json).unwrap(), "");

    #[test]
    fn test_session_startstop() {
        let envelopes = with_captured_envelopes_options(
            || {
                sentry::start_session();
                std::thread::sleep(std::time::Duration::from_millis(10));
                sentry::end_session();
            },
            ClientOptions {
                ..Default::default()
            },
        );
        assert_eq!(envelopes.len(), 1);
        dbg!(envelopes);
    }

    #[test]
    fn test_session_error() {
        let envelopes = with_captured_envelopes_options(
            || {
                sentry::start_session();

                let err = "NaN".parse::<usize>().unwrap_err();
                sentry::capture_error(&err);

                sentry::end_session();
            },
            ClientOptions {
                ..Default::default()
            },
        );
        assert_eq!(envelopes.len(), 2);
        dbg!(envelopes);
    }
}
