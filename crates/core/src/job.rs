//! Asynchronous simulation job identity, status, and progress.
//!
//! These are Domain Layer value objects describing *where a job is*, not how it
//! is stored or executed. The string forms produced by [`JobStatus::as_str`] are
//! the contract shared with the `job_status` `PostgreSQL` enum declared in
//! `migrations/0004_simulation_jobs.sql`.

use std::fmt;
use std::str::FromStr;

use time::OffsetDateTime;
use uuid::Uuid;

/// Unique identifier of a simulation job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(Uuid);

impl JobId {
    /// Generates a fresh job identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::job::JobId;
    ///
    /// let a = JobId::new();
    /// let b = JobId::new();
    /// assert_ne!(a, b);
    /// assert!(!a.to_string().is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        return Self(Uuid::new_v4());
    }

    /// Wraps an existing UUID as a job identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::job::JobId;
    /// use uuid::Uuid;
    ///
    /// let raw = Uuid::nil();
    /// assert_eq!(JobId::from_uuid(raw).as_uuid(), raw);
    /// ```
    #[must_use]
    pub fn from_uuid(id: Uuid) -> Self {
        return Self(id);
    }

    /// Returns the underlying UUID.
    #[must_use]
    pub fn as_uuid(self) -> Uuid {
        return self.0;
    }
}

impl Default for JobId {
    fn default() -> Self {
        return Self::new();
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return self.0.fmt(f);
    }
}

impl FromStr for JobId {
    type Err = JobError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        return Uuid::parse_str(value).map(Self).map_err(|_| {
            return JobError::MalformedId;
        });
    }
}

/// Lifecycle state of a simulation job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobStatus {
    /// Accepted and waiting for a worker to claim it.
    Queued,

    /// Claimed by a worker and currently executing.
    Running,

    /// Finished successfully; results are available.
    Completed,

    /// Finished unsuccessfully; an error message is available.
    Failed,

    /// Cancelled by the owner before it could complete.
    Cancelled,
}

impl JobStatus {
    /// Returns the database representation of this status.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::job::JobStatus;
    ///
    /// assert_eq!(JobStatus::Queued.as_str(), "queued");
    /// assert_eq!(JobStatus::Cancelled.as_str(), "cancelled");
    /// ```
    #[must_use]
    pub fn as_str(self) -> &'static str {
        return match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        };
    }

    /// Parses a status from its database representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::job::JobStatus;
    ///
    /// assert_eq!(JobStatus::parse("running"), Some(JobStatus::Running));
    /// assert_eq!(JobStatus::parse("elsewhere"), None);
    /// ```
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        return match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        };
    }

    /// Returns true when the job has reached a state it can never leave.
    ///
    /// Terminal jobs are never claimed by a worker and can never be cancelled.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::job::JobStatus;
    ///
    /// assert!(JobStatus::Completed.is_terminal());
    /// assert!(!JobStatus::Queued.is_terminal());
    /// ```
    #[must_use]
    pub fn is_terminal(self) -> bool {
        return matches!(self, Self::Completed | Self::Failed | Self::Cancelled);
    }

    /// Returns true when a job in this state may still be cancelled.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::job::JobStatus;
    ///
    /// assert!(JobStatus::Running.is_cancellable());
    /// assert!(!JobStatus::Failed.is_cancellable());
    /// ```
    #[must_use]
    pub fn is_cancellable(self) -> bool {
        return !self.is_terminal();
    }
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f.write_str(self.as_str());
    }
}

/// How far a job has progressed through its universes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobProgress {
    /// Total universes the job intends to simulate.
    pub universes_total: usize,

    /// Universes completed so far.
    pub universes_completed: usize,

    /// When the last checkpoint was written, if any.
    pub last_checkpoint_at: Option<OffsetDateTime>,
}

impl JobProgress {
    /// Creates a progress record for a job that has not yet started.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::job::JobProgress;
    ///
    /// let progress = JobProgress::new(1_000);
    /// assert_eq!(progress.universes_total, 1_000);
    /// assert_eq!(progress.universes_completed, 0);
    /// ```
    #[must_use]
    pub fn new(universes_total: usize) -> Self {
        return Self {
            universes_total,
            universes_completed: 0,
            last_checkpoint_at: None,
        };
    }

    /// Returns completion as a fraction in `0.0..=1.0`.
    ///
    /// A job with no universes is defined as fully complete, so callers polling
    /// for `1.0` never spin forever on a degenerate request.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::job::JobProgress;
    ///
    /// let mut progress = JobProgress::new(4);
    /// progress.universes_completed = 1;
    /// assert!((progress.fraction() - 0.25).abs() < f64::EPSILON);
    ///
    /// assert!((JobProgress::new(0).fraction() - 1.0).abs() < f64::EPSILON);
    /// ```
    #[must_use]
    pub fn fraction(&self) -> f64 {
        if self.universes_total == 0 {
            return 1.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let fraction = self.universes_completed as f64 / self.universes_total as f64;
        return fraction.clamp(0.0, 1.0);
    }
}

/// Errors raised when working with job identifiers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JobError {
    /// The supplied job identifier was not a valid UUID.
    #[error("job identifier must be a valid UUID")]
    MalformedId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_ids_are_unique() {
        assert_ne!(JobId::new(), JobId::new());
        assert_eq!(JobId::from_uuid(Uuid::nil()), JobId::from_uuid(Uuid::nil()));
    }

    #[test]
    fn job_id_round_trips_through_string() {
        let id = JobId::new();
        let parsed: JobId = id.to_string().parse().unwrap();
        assert_eq!(parsed, id);
        assert_eq!(parsed.as_uuid(), id.as_uuid());
    }

    #[test]
    fn malformed_job_id_is_rejected() {
        assert_eq!("not-a-uuid".parse::<JobId>(), Err(JobError::MalformedId));
        assert_eq!("".parse::<JobId>(), Err(JobError::MalformedId));
    }

    #[test]
    fn status_round_trips_through_string() {
        for status in [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Completed,
            JobStatus::Failed,
            JobStatus::Cancelled,
        ] {
            assert_eq!(JobStatus::parse(status.as_str()), Some(status));
            assert!(!status.as_str().is_empty());
        }
    }

    #[test]
    fn terminal_states_are_not_cancellable() {
        assert!(JobStatus::Completed.is_terminal());
        assert!(!JobStatus::Completed.is_cancellable());
    }

    #[test]
    fn active_states_are_cancellable() {
        assert!(JobStatus::Queued.is_cancellable());
        assert!(JobStatus::Running.is_cancellable());
    }

    #[test]
    fn fraction_reports_partial_completion() {
        let mut progress = JobProgress::new(200);
        progress.universes_completed = 50;
        assert!((progress.fraction() - 0.25).abs() < f64::EPSILON);
        assert_eq!(progress.universes_total, 200);
    }

    #[test]
    fn empty_job_is_complete() {
        let progress = JobProgress::new(0);
        assert!((progress.fraction() - 1.0).abs() < f64::EPSILON);
        assert_eq!(progress.universes_completed, 0);
    }

    #[test]
    fn fraction_never_exceeds_one() {
        let mut progress = JobProgress::new(10);
        progress.universes_completed = 999;
        assert!((progress.fraction() - 1.0).abs() < f64::EPSILON);
        assert!(progress.fraction() <= 1.0);
    }
}
