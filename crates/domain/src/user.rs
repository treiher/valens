use derive_more::Deref;
use std::fmt;
use uuid::Uuid;

use crate::{CreateError, DeleteError, Name, ReadError, UpdateError, ValidationError};

#[allow(async_fn_in_trait)]
pub trait UserService: Send + Sync + 'static {
    async fn get_users(&self) -> Result<Vec<User>, ReadError>;
    async fn create_user(
        &self,
        name: Name,
        sex: Sex,
        height: Option<u8>,
    ) -> Result<User, CreateError>;
    async fn replace_user(&self, user: User) -> Result<User, UpdateError>;
    async fn delete_user(&self, id: UserID) -> Result<(), DeleteError>;

    async fn validate_user_name(&self, name: &str, id: UserID) -> Result<Name, ValidationError> {
        match Name::new(name) {
            Ok(name) => match self.get_users().await {
                Ok(users) => {
                    if users.iter().all(|u| u.id == id || u.name != name) {
                        Ok(name)
                    } else {
                        Err(ValidationError::Conflict("name".to_string()))
                    }
                }
                Err(err) => Err(ValidationError::Other(err.into())),
            },
            Err(err) => Err(ValidationError::Other(err.into())),
        }
    }

    fn validate_user_height(&self, height: &str) -> Result<Option<u8>, ValidationError> {
        if height.trim().is_empty() {
            return Ok(None);
        }
        match height.trim().parse::<u8>() {
            Ok(parsed_height) if parsed_height > 0 => Ok(Some(parsed_height)),
            _ => Err(ValidationError::Other(
                "Height must be a whole number between 1 and 255".into(),
            )),
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait UserRepository: Send + Sync + 'static {
    async fn read_users(&self) -> Result<Vec<User>, ReadError>;
    async fn create_user(
        &self,
        name: Name,
        sex: Sex,
        height: Option<u8>,
    ) -> Result<User, CreateError>;
    async fn replace_user(&self, user: User) -> Result<User, UpdateError>;
    async fn delete_user(&self, id: UserID) -> Result<(), DeleteError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: UserID,
    pub name: Name,
    pub sex: Sex,
    pub height: Option<u8>,
}

#[derive(Deref, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserID(Uuid);

impl UserID {
    #[must_use]
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }

    #[must_use]
    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }
}

impl From<Uuid> for UserID {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<u128> for UserID {
    fn from(value: u128) -> Self {
        Self(Uuid::from_bytes(value.to_be_bytes()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sex {
    FEMALE,
    MALE,
}

impl From<u8> for Sex {
    fn from(value: u8) -> Self {
        match value {
            0 => Sex::FEMALE,
            _ => Sex::MALE,
        }
    }
}

impl From<&str> for Sex {
    fn from(value: &str) -> Self {
        match value {
            "female" => Sex::FEMALE,
            _ => Sex::MALE,
        }
    }
}

impl fmt::Display for Sex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Sex::FEMALE => "female",
                Sex::MALE => "male",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    struct TestService;

    impl UserService for TestService {
        async fn get_users(&self) -> Result<Vec<User>, ReadError> {
            unimplemented!()
        }

        async fn create_user(&self, _: Name, _: Sex, _: Option<u8>) -> Result<User, CreateError> {
            unimplemented!()
        }

        async fn replace_user(&self, _: User) -> Result<User, UpdateError> {
            unimplemented!()
        }

        async fn delete_user(&self, _: UserID) -> Result<(), DeleteError> {
            unimplemented!()
        }
    }

    #[test]
    fn test_user_id_nil() {
        assert!(UserID::nil().is_nil());
        assert_eq!(UserID::nil(), UserID::default());
    }

    #[rstest]
    #[case("", Ok(None))]
    #[case("  ", Ok(None))]
    #[case("180", Ok(Some(180)))]
    #[case(" 175 ", Ok(Some(175)))]
    #[case("0", Err("Height must be a whole number between 1 and 255"))]
    #[case("-180", Err("Height must be a whole number between 1 and 255"))]
    #[case("175.5", Err("Height must be a whole number between 1 and 255"))]
    #[case("1000", Err("Height must be a whole number between 1 and 255"))]
    #[case("abc", Err("Height must be a whole number between 1 and 255"))]
    fn test_validate_user_height(#[case] input: &str, #[case] expected: Result<Option<u8>, &str>) {
        assert_eq!(
            TestService
                .validate_user_height(input)
                .map_err(|err| err.to_string()),
            expected.map_err(str::to_string)
        );
    }

    #[rstest]
    #[case(0, Sex::FEMALE)]
    #[case(1, Sex::MALE)]
    #[case(2, Sex::MALE)]
    fn test_sex_from_u8(#[case] value: u8, #[case] expected: Sex) {
        assert_eq!(Sex::from(value), expected);
    }

    #[rstest]
    #[case(Sex::FEMALE, "female")]
    #[case(Sex::MALE, "male")]
    fn test_sex_display(#[case] sex: Sex, #[case] string: &str) {
        assert_eq!(sex.to_string(), string);
    }
}
