//! Generic, domain-agnostic UI building blocks.

pub mod drag_and_drop;
pub mod element;
pub mod form;

pub fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_character_is_capitalized() {
        assert_eq!(capitalized("server unreachable"), "Server unreachable");
        assert_eq!(
            capitalized("name must be 64 characters or fewer (65 > 64)"),
            "Name must be 64 characters or fewer (65 > 64)"
        );
        assert_eq!(
            capitalized("RPE must be a decimal"),
            "RPE must be a decimal"
        );
    }

    /// Display boundaries also receive text that is already capitalized.
    #[test]
    fn capitalizing_is_idempotent() {
        assert_eq!(capitalized("Server unreachable"), "Server unreachable");
    }

    #[test]
    fn empty_text_stays_empty() {
        assert_eq!(capitalized(""), "");
    }
}
