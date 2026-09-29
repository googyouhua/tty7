use serde::{Deserialize, Serialize};

/// WindTerm-style OneKey entry: a saved credential that can be manually filled
/// into any shell pane (see `specs/onekey-autofill/spec.md`).
#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct OneKeyEntry {
    pub id: String,
    pub kind: OneKeyKind,
    pub title: String,
    pub username: String,
    /// Plaintext in `config.json` by explicit user decision (D5/D8); the
    /// Settings UI must surface the secure-environment warning.
    pub password: String,
    pub host_binding: Option<String>,
}

/// Redacted on purpose: a `Debug` dump must never carry the secret, following
/// the existing `without_secrets` pattern for SSH specs.
impl std::fmt::Debug for OneKeyEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OneKeyEntry")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("title", &self.title)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("host_binding", &self.host_binding)
            .finish()
    }
}

impl Default for OneKeyEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: OneKeyKind::Account,
            title: String::new(),
            username: String::new(),
            password: String::new(),
            host_binding: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OneKeyKind {
    #[default]
    Account,
    SshBound,
}

/// What the picker sends once an entry is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneKeyFill {
    Username,
    Password,
    UsernameAndPassword,
}

impl OneKeyEntry {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.title.trim().is_empty() || self.title.chars().count() > 64 {
            return Err("title must be 1-64 chars");
        }
        Ok(())
    }

    /// Bytes to send for a fill choice. `UsernameAndPassword` types the
    /// username, presses Enter (`\r`), then types the password with no final
    /// submit (D9). Returns empty vec only when there is nothing to send
    /// (both fields empty); callers treat cancel separately (zero bytes).
    pub fn fill_bytes(&self, fill: OneKeyFill) -> Vec<u8> {
        match fill {
            OneKeyFill::Username => self.username.clone().into_bytes(),
            OneKeyFill::Password => self.password.clone().into_bytes(),
            OneKeyFill::UsernameAndPassword => {
                let mut out = self.username.clone().into_bytes();
                out.push(b'\r');
                out.extend_from_slice(self.password.as_bytes());
                out
            }
        }
    }

    pub fn matches_filter(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        self.title.to_lowercase().contains(&q)
            || self.username.to_lowercase().contains(&q)
            || self
                .host_binding
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains(&q)
    }
}

/// Security warning text shared by Settings and docs (brief A5, spec §6).
pub const ONEKEY_SECURITY_WARNING: &str = "OneKey secrets are stored as plaintext in config.json. Use OneKey autofill only in a SECURE environment: anyone who can read the file can recover them, and listing or sending an entry exposes its secret. A fake login screen can steal filled credentials.";

pub fn titles_unique(entries: &[OneKeyEntry]) -> bool {
    let mut seen = std::collections::HashSet::new();
    for e in entries {
        if !seen.insert(e.title.trim().to_lowercase()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> OneKeyEntry {
        OneKeyEntry {
            id: "1".to_string(),
            kind: OneKeyKind::Account,
            title: "prod".to_string(),
            username: "alice".to_string(),
            password: "s3cret".to_string(),
            host_binding: Some("10.0.0.5".to_string()),
        }
    }

    #[test]
    fn username_fill_sends_only_username() {
        assert_eq!(entry().fill_bytes(OneKeyFill::Username), b"alice");
    }

    #[test]
    fn password_fill_sends_only_password() {
        assert_eq!(entry().fill_bytes(OneKeyFill::Password), b"s3cret");
    }

    #[test]
    fn sequence_is_username_enter_password_without_final_submit() {
        assert_eq!(
            entry().fill_bytes(OneKeyFill::UsernameAndPassword),
            b"alice\rs3cret"
        );
    }

    #[test]
    fn filter_matches_title_username_and_binding() {
        let e = entry();
        assert!(e.matches_filter(""));
        assert!(e.matches_filter("prod"));
        assert!(e.matches_filter("ALICE"));
        assert!(e.matches_filter("10.0.0"));
        assert!(!e.matches_filter("bob"));
    }

    #[test]
    fn title_validation_rejects_empty_and_long() {
        let mut e = entry();
        e.title = String::new();
        assert!(e.validate().is_err());
        e.title = "x".repeat(65);
        assert!(e.validate().is_err());
        e.title = "ok".to_string();
        assert!(e.validate().is_ok());
    }

    #[test]
    fn duplicate_titles_detected_case_insensitively() {
        let mut a = entry();
        let mut b = entry();
        b.id = "2".to_string();
        b.title = "PROD".to_string();
        assert!(!titles_unique(&[a.clone(), b.clone()]));
        a.title = "other".to_string();
        assert!(titles_unique(&[a, b]));
    }

    #[test]
    fn serde_round_trips_through_config_json() {
        let e = entry();
        let json = serde_json::to_string(&e).unwrap();
        let back: OneKeyEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn debug_never_carries_the_secret() {
        let dump = format!("{:?}", entry());
        assert!(!dump.contains("s3cret"), "password must be redacted");
        assert!(dump.contains("<redacted>"));
        assert!(dump.contains("alice"));
    }
}
