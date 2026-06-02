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
            ReadError::Storage(storage) => SyncError::Storage(storage),
            ReadError::Other(other) => SyncError::Other(other),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

#[derive(thiserror::Error, Debug)]
pub enum CreateError {
    #[error("conflict")]
    Conflict,
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

impl From<UpdateError> for CreateError {
    fn from(value: UpdateError) -> Self {
        match value {
            UpdateError::Conflict => CreateError::Conflict,
            UpdateError::Storage(storage) => CreateError::Storage(storage),
            UpdateError::Other(other) => CreateError::Other(other),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
    #[error("conflict")]
    Conflict,
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

#[derive(thiserror::Error, Debug)]
pub enum DeleteError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
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
            SyncError::from(ReadError::Storage(StorageError::NoSession)),
            SyncError::Storage(StorageError::NoSession)
        ));
        assert!(matches!(
            SyncError::from(ReadError::Other("foo".into())),
            SyncError::Other(error) if error.to_string() == "foo"
        ));
    }

    #[test]
    fn test_create_error_from_update_error() {
        assert!(matches!(
            CreateError::from(UpdateError::Conflict),
            CreateError::Conflict
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
        assert!(!CreateError::Conflict.recoverable());

        assert!(UpdateError::Storage(StorageError::Timeout).recoverable());
        assert!(!UpdateError::Conflict.recoverable());

        assert!(DeleteError::Storage(StorageError::NoConnection).recoverable());
        assert!(!DeleteError::Other("foo".into()).recoverable());

        assert!(SyncError::Storage(StorageError::NoConnection).recoverable());
        assert!(!SyncError::Other("foo".into()).recoverable());
    }
}
