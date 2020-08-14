//! Release Health Sessions
//!
//! https://develop.sentry.dev/sdk/sessions/
//!

use std::fmt;
use std::str;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use thiserror::Error;

use crate::types::{DateTime, Utc, Uuid};

/// An error used when parsing the session `status`.
#[derive(Debug, Error)]
#[error("invalid session status")]
pub struct ParseSessionStatusError;

/// Represents the status of a session.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SessionStatus {
    Ok,
    Crashed,
    Abnormal,
    Exited,
}

impl str::FromStr for SessionStatus {
    type Err = ParseSessionStatusError;

    fn from_str(string: &str) -> Result<SessionStatus, Self::Err> {
        Ok(match string {
            "ok" => SessionStatus::Ok,
            "crashed" => SessionStatus::Crashed,
            "abnormal" => SessionStatus::Abnormal,
            "exited" => SessionStatus::Exited,
            _ => return Err(ParseSessionStatusError),
        })
    }
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

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, serde::Serialize)]
pub struct Session {
    pub(crate) session_id: Uuid,
    pub(crate) status: SessionStatus,
    // this is atomic to avoid having a writer lock on the scope stack
    pub(crate) errors: AtomicUsize,
    #[serde(skip)]
    pub(crate) started: Instant,
    #[serde(rename = "started")]
    started_utc: DateTime<Utc>,
    pub(crate) duration: Option<f64>,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) init: bool,
}

impl Session {
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            status: SessionStatus::Ok,
            errors: AtomicUsize::new(0),
            started: Instant::now(),
            started_utc: Utc::now(),
            duration: None,
            init: true,
        }
    }

    pub(crate) fn record_errors(&self, errors: usize) {
        self.errors.fetch_add(errors, Ordering::SeqCst);
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            session_id: self.session_id,
            status: self.status,
            errors: AtomicUsize::new(self.errors.load(Ordering::SeqCst)),
            started: self.started.clone(),
            started_utc: self.started_utc.clone(),
            duration: self.duration.clone(),
            init: self.init,
        }
    }
}

// impl serde::Serialize for Session {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut state = serializer.serialize_struct("Color", 3)?;
//         state.serialize_field("r", &self.r)?;
//         state.serialize_field("g", &self.g)?;
//         state.serialize_field("b", &self.b)?;
//         state.end()
//         serializer.serialize_str(&self.to_string())
//     }
// }

impl Default for Session {
    fn default() -> Self {
        Session::new()
    }
}
