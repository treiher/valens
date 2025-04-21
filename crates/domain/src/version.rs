use crate::ReadError;

#[allow(async_fn_in_trait)]
pub trait VersionRepository {
    async fn read_version(&self) -> Result<String, ReadError>;
}
