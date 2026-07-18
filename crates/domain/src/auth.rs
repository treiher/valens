use chrono::NaiveDate;
use derive_more::Deref;
use uuid::Uuid;

use crate::{CreateError, DeleteError, Name, ReadError, UpdateError, User, UserID};

#[allow(async_fn_in_trait)]
pub trait AuthService: Send + Sync + 'static {
    /// Determine the authentication methods offered by the server.
    async fn get_auth_methods(&self) -> Result<Vec<AuthMethod>, ReadError>;
    /// Log in with a passkey using the discoverable credential flow.
    async fn login_with_passkey(&self) -> Result<User, ReadError>;
    /// Register a new passkey for the current user.
    async fn register_passkey(&self) -> Result<Passkey, CreateError>;
    async fn get_passkeys(&self, user_id: UserID) -> Result<Vec<Passkey>, ReadError>;
    async fn rename_passkey(
        &self,
        user_id: UserID,
        id: PasskeyID,
        label: Name,
    ) -> Result<Passkey, UpdateError>;
    async fn delete_passkey(&self, user_id: UserID, id: PasskeyID) -> Result<(), DeleteError>;
    /// Create a one-time login link for the user and return its URL.
    async fn create_login_link(&self, user_id: UserID) -> Result<String, CreateError>;
    /// Log in with the token of a one-time login link.
    async fn redeem_login_link(&self, token: String) -> Result<User, ReadError>;
}

#[allow(async_fn_in_trait)]
pub trait AuthRepository: Send + Sync + 'static {
    /// Determine the authentication methods offered by the server.
    async fn read_auth_methods(&self) -> Result<Vec<AuthMethod>, ReadError>;
    /// Log in with a passkey using the discoverable credential flow.
    async fn login_with_passkey(&self) -> Result<User, ReadError>;
    /// Register a new passkey for the current user.
    async fn register_passkey(&self) -> Result<Passkey, CreateError>;
    async fn read_passkeys(&self, user_id: UserID) -> Result<Vec<Passkey>, ReadError>;
    async fn rename_passkey(
        &self,
        user_id: UserID,
        id: PasskeyID,
        label: Name,
    ) -> Result<Passkey, UpdateError>;
    async fn delete_passkey(&self, user_id: UserID, id: PasskeyID) -> Result<(), DeleteError>;
    /// Create a one-time login link for the user and return its URL.
    async fn create_login_link(&self, user_id: UserID) -> Result<String, CreateError>;
    /// Log in with the token of a one-time login link.
    async fn redeem_login_link(&self, token: String) -> Result<User, ReadError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    Passkey,
    Username,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passkey {
    pub id: PasskeyID,
    pub label: Name,
    pub created: NaiveDate,
    pub last_used: Option<NaiveDate>,
}

#[derive(Deref, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PasskeyID(Uuid);

impl From<Uuid> for PasskeyID {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<u128> for PasskeyID {
    fn from(value: u128) -> Self {
        Self(Uuid::from_bytes(value.to_be_bytes()))
    }
}
