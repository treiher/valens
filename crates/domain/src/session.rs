use crate::{DeleteError, Name, ReadError, User};

#[allow(async_fn_in_trait)]
pub trait SessionService {
    async fn request_session(&self, name: Name) -> Result<User, ReadError>;
    async fn get_session(&self) -> Result<User, ReadError>;
    async fn delete_session(&self) -> Result<(), DeleteError>;
}

#[allow(async_fn_in_trait)]
pub trait SessionRepository {
    async fn request_session(&self, name: Name) -> Result<User, ReadError>;
    async fn initialize_session(&self) -> Result<User, ReadError>;
    async fn delete_session(&self) -> Result<(), DeleteError>;
}
