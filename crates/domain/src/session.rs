use crate::{DeleteError, Name, ReadError, SyncError, User};

/// Extent to which a session has been removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignOut {
    /// The session has been removed together with the data stored on the device.
    Complete,
    /// The session has been removed on the server, but data stored on the device was retained.
    DataRetained,
}

#[allow(async_fn_in_trait)]
pub trait SessionService {
    async fn request_session(&self, name: Name) -> Result<User, ReadError>;
    async fn get_session(&self) -> Result<User, ReadError>;
    /// Update the locally stored session with the session on the server.
    ///
    /// Returns `None` if no session exists on the server.
    async fn sync_session(&self) -> Result<Option<User>, SyncError>;
    async fn delete_session(&self) -> Result<SignOut, DeleteError>;
}

#[allow(async_fn_in_trait)]
pub trait SessionRepository {
    async fn request_session(&self, name: Name) -> Result<User, ReadError>;
    async fn initialize_session(&self) -> Result<User, ReadError>;
    /// Update the locally stored session with the session on the server.
    ///
    /// Returns `None` if no session exists on the server.
    async fn sync_session(&self) -> Result<Option<User>, SyncError>;
    async fn delete_session(&self) -> Result<SignOut, DeleteError>;
}
