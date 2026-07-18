#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

impl From<ReadError> for SyncError {
    fn from(value: ReadError) -> Self {
        match value {
            ReadError::NotFound => SyncError::Other("not found".into()),
            ReadError::Unauthorized(reason) | ReadError::Forbidden(reason) => {
                SyncError::Other(reason.into())
            }
            ReadError::Storage(storage) => SyncError::Storage(storage),
            ReadError::Other(other) => SyncError::Other(other),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

#[derive(thiserror::Error, Debug)]
pub enum CreateError {
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

impl From<UpdateError> for CreateError {
    fn from(value: UpdateError) -> Self {
        match value {
            UpdateError::Conflict(reason) => CreateError::Conflict(reason),
            UpdateError::Forbidden(reason) => CreateError::Forbidden(reason),
            UpdateError::Storage(storage) => CreateError::Storage(storage),
            UpdateError::Other(other) => CreateError::Other(other),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

impl From<ReadError> for UpdateError {
    fn from(value: ReadError) -> Self {
        match value {
            ReadError::NotFound => UpdateError::Other("not found".into()),
            ReadError::Unauthorized(reason) => UpdateError::Other(reason.into()),
            ReadError::Forbidden(reason) => UpdateError::Forbidden(reason),
            ReadError::Storage(storage) => UpdateError::Storage(storage),
            ReadError::Other(other) => UpdateError::Other(other),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DeleteError {
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

impl From<ReadError> for DeleteError {
    fn from(value: ReadError) -> Self {
        match value {
            ReadError::NotFound => DeleteError::Other("not found".into()),
            ReadError::Unauthorized(reason) => DeleteError::Other(reason.into()),
            ReadError::Forbidden(reason) => DeleteError::Forbidden(reason),
            ReadError::Storage(storage) => DeleteError::Storage(storage),
            ReadError::Other(other) => DeleteError::Other(other),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("no connection")]
    NoConnection,
    #[error("no session")]
    NoSession,
    #[error("timeout")]
    Timeout,
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Entry with this {0} already exists")]
    Conflict(String),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

/// Whether an error reflects a transient or otherwise expected condition rather than a genuine
/// fault. `NoConnection` and `Timeout` may succeed on retry; `NoSession` reflects the absence of an
/// active session rather than a failure.
pub trait Recoverable {
    fn recoverable(&self) -> bool;
}

impl Recoverable for StorageError {
    fn recoverable(&self) -> bool {
        matches!(
            self,
            StorageError::NoConnection | StorageError::NoSession | StorageError::Timeout
        )
    }
}

impl Recoverable for ReadError {
    fn recoverable(&self) -> bool {
        matches!(self, ReadError::Storage(err) if err.recoverable())
    }
}

impl Recoverable for CreateError {
    fn recoverable(&self) -> bool {
        matches!(self, CreateError::Storage(err) if err.recoverable())
    }
}

impl Recoverable for UpdateError {
    fn recoverable(&self) -> bool {
        matches!(self, UpdateError::Storage(err) if err.recoverable())
    }
}

impl Recoverable for DeleteError {
    fn recoverable(&self) -> bool {
        matches!(self, DeleteError::Storage(err) if err.recoverable())
    }
}

impl Recoverable for SyncError {
    fn recoverable(&self) -> bool {
        matches!(self, SyncError::Storage(err) if err.recoverable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_error_from_read_error() {
        assert!(matches!(
            SyncError::from(ReadError::Unauthorized("foo".to_string())),
            SyncError::Other(error) if error.to_string() == "foo"
        ));
        assert!(matches!(
            SyncError::from(ReadError::Forbidden("foo".to_string())),
            SyncError::Other(error) if error.to_string() == "foo"
        ));
        assert!(matches!(
            SyncError::from(ReadError::Storage(StorageError::NoSession)),
            SyncError::Storage(StorageError::NoSession)
        ));
        assert!(matches!(
            SyncError::from(ReadError::Other("foo".into())),
            SyncError::Other(error) if error.to_string() == "foo"
        ));
    }

    #[test]
    fn test_update_error_from_read_error() {
        assert!(matches!(
            UpdateError::from(ReadError::NotFound),
            UpdateError::Other(error) if error.to_string() == "not found"
        ));
        assert!(matches!(
            UpdateError::from(ReadError::Unauthorized("foo".to_string())),
            UpdateError::Other(error) if error.to_string() == "foo"
        ));
        assert!(matches!(
            UpdateError::from(ReadError::Forbidden("foo".to_string())),
            UpdateError::Forbidden(reason) if reason == "foo"
        ));
        assert!(matches!(
            UpdateError::from(ReadError::Storage(StorageError::NoSession)),
            UpdateError::Storage(StorageError::NoSession)
        ));
        assert!(matches!(
            UpdateError::from(ReadError::Other("foo".into())),
            UpdateError::Other(error) if error.to_string() == "foo"
        ));
    }

    #[test]
    fn test_create_error_from_update_error() {
        assert!(matches!(
            CreateError::from(UpdateError::Conflict("foo".to_string())),
            CreateError::Conflict(reason) if reason == "foo"
        ));
        assert!(matches!(
            CreateError::from(UpdateError::Forbidden("foo".to_string())),
            CreateError::Forbidden(reason) if reason == "foo"
        ));
        assert!(matches!(
            CreateError::from(UpdateError::Storage(StorageError::NoSession)),
            CreateError::Storage(StorageError::NoSession)
        ));
        assert!(matches!(
            CreateError::from(UpdateError::Other("foo".into())),
            CreateError::Other(error) if error.to_string() == "foo"
        ));
    }

    #[test]
    fn test_storage_error_recoverable() {
        assert!(StorageError::NoConnection.recoverable());
        assert!(StorageError::NoSession.recoverable());
        assert!(StorageError::Timeout.recoverable());
        assert!(!StorageError::Other("foo".into()).recoverable());
    }

    #[test]
    fn test_error_recoverable_delegates_to_storage() {
        assert!(ReadError::Storage(StorageError::NoConnection).recoverable());
        assert!(!ReadError::Storage(StorageError::Other("foo".into())).recoverable());
        assert!(!ReadError::NotFound.recoverable());

        assert!(CreateError::Storage(StorageError::NoSession).recoverable());
        assert!(!CreateError::Conflict("foo".to_string()).recoverable());

        assert!(UpdateError::Storage(StorageError::Timeout).recoverable());
        assert!(!UpdateError::Conflict("foo".to_string()).recoverable());

        assert!(DeleteError::Storage(StorageError::NoConnection).recoverable());
        assert!(!DeleteError::Other("foo".into()).recoverable());

        assert!(SyncError::Storage(StorageError::NoConnection).recoverable());
        assert!(!SyncError::Other("foo".into()).recoverable());
    }
}
