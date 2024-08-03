use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

const FORBIDDEN_CHARACTERS: [char; 9] = ['/', '\\', '(', ')', '"', '{', '}', '<', '>'];

impl SubscriberName {
    // returns a SubscriberName instance if the constraints are satisfied
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty_or_whitespace = s.trim().is_empty();

        // a grapheme is a "user-perceived" character (it can be formed by two character but is read as one, like å is `a` and `̊``).
        // graphemes returns an iterator over the graphemes of the input.
        // `true` specifies that we want to use the extended grapheme definition set, recommended.
        let is_too_log = s.graphemes(true).count() > 256;

        let contains_forbidden_characters = s.chars().any(|g| FORBIDDEN_CHARACTERS.contains(&g));

        if is_empty_or_whitespace || is_too_log || contains_forbidden_characters {
            Err(format!("{} is not a valid subscriber name", s))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::subscriber_name::{SubscriberName, FORBIDDEN_CHARACTERS};
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_name_is_valid() {
        let name = "a".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_257_grapheme_name_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_name_is_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_name_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_invalid_character_are_rejected() {
        for name in &FORBIDDEN_CHARACTERS {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_succesfully() {
        let name = "Alpha Centauri".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
