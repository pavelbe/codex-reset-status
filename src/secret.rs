pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose_for_header(&self) -> &str {
        &self.0
    }

    pub fn redact(&self, text: &str) -> String {
        if self.0.is_empty() {
            return text.to_owned();
        }
        text.replace(&self.0, "[redacted]")
    }
}

#[cfg(test)]
mod tests {
    use super::Secret;

    #[test]
    fn removes_the_actual_secret() {
        let token = Secret::new("secret-token-value".to_owned());
        let input = "server echoed secret-token-value";
        assert!(input.contains(token.expose_for_header()));

        let output = token.redact(input);
        assert_eq!(output, "server echoed [redacted]");
        assert!(!output.contains(token.expose_for_header()));
    }
}
