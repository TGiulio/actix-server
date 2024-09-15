use rand::{distributions::Alphanumeric, thread_rng, Rng, RngCore};
use regex::Regex;

const TOKEN_LENGTH: usize = 25;

#[derive(Debug)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    // returns a SubscriptionToken instance if the constraints are satisfied
    pub fn parse(s: String) -> Result<Self, String> {
        let token_regex = Regex::new(&format!(r"(?m)^[a-zA-Z0-9]{{{}}}$", TOKEN_LENGTH)).unwrap();
        if token_regex.is_match(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid token", s))
        }
    }

    pub fn new_token_string() -> String {
        let mut rng = thread_rng();
        std::iter::repeat_with(|| rng.sample(Alphanumeric))
            .map(char::from)
            .take(TOKEN_LENGTH)
            .collect()
    }

    pub fn new() -> Self {
        let token = Self::new_token_string();
        SubscriptionToken::parse(token).unwrap() // it should always be parsed correctly
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::subscription_token::{SubscriptionToken, TOKEN_LENGTH};
    use claim::assert_err;
    use rand::{
        distributions::{Alphanumeric, DistString},
        RngCore,
    };

    #[test]
    fn empty_token_is_rejected() {
        let token = "".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn too_long_token_is_rejected() {
        let token = "KYu7R2TPDCAy1rT141uOExlVVg".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn too_short_token_is_rejected() {
        let token = "KYu7R2TPDCAy1rT141Gcy8rI".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn invalid_token_is_rejected() {
        let token = "KYu7R2TPD_CAy1rT141Gcy8I".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    fn valid_token_are_parsed_succesfully() {
        for i in (0..100) {
            let valid_token = SubscriptionToken::new_token_string();
            let _ = SubscriptionToken::parse(valid_token).is_ok();
        }
    }
}
