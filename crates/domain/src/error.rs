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
            ReadError::Storage(storage) => SyncError::Storage(storage),
            ReadError::Other(other) => SyncError::Other(other),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
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
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
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
}
