use derive_more::{AsRef, Display};

#[derive(AsRef, Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Name(String);

impl Name {
    pub fn new(name: &str) -> Result<Self, NameError> {
        let trimmed_name = name.trim();

        if trimmed_name.is_empty() {
            return Err(NameError::Empty);
        }

        let len = trimmed_name.len();

        if len > 64 {
            return Err(NameError::TooLong(len));
        }

        Ok(Name(trimmed_name.to_string()))
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum NameError {
    #[error("Name must not be empty")]
    Empty,
    #[error("Name must be 64 characters or fewer ({0} > 64)")]
    TooLong(usize),
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("Alice", Ok(Name("Alice".to_string())))]
    #[case("  Bob  ", Ok(Name("Bob".to_string())))]
    #[case("", Err(NameError::Empty))]
    #[case(
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        Err(NameError::TooLong(65))
    )]
    fn test_name_new(#[case] name: &str, #[case] expected: Result<Name, NameError>) {
        assert_eq!(Name::new(name), expected);
    }
}
