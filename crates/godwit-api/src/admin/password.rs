use godwit_core::PasswordPolicy;
use godwit_auth::api_keys::{hash_password, verify_password};

mod common {
    use std::collections::HashSet;
    use std::sync::OnceLock;
    static LIST: &str = include_str!("../../assets/common_passwords.txt");
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    pub fn lookup(pw: &str) -> bool {
        SET.get_or_init(|| LIST.lines().map(|l| l.trim().to_string()).collect())
            .contains(&pw.to_ascii_lowercase())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PolicyError {
    TooShort,
    NeedsUpper,
    NeedsLower,
    NeedsDigit,
    NeedsSymbol,
    CommonPassword,
    Reused,
}

pub fn validate_password(
    policy: &PasswordPolicy,
    password: &str,
    history: &[String],
) -> Result<(), PolicyError> {
    if policy.block_common && common::lookup(password) {
        return Err(PolicyError::CommonPassword);
    }
    if (password.chars().count() as u32) < policy.min_length {
        return Err(PolicyError::TooShort);
    }
    if policy.require_upper && !password.chars().any(|c| c.is_uppercase()) {
        return Err(PolicyError::NeedsUpper);
    }
    if policy.require_lower && !password.chars().any(|c| c.is_lowercase()) {
        return Err(PolicyError::NeedsLower);
    }
    if policy.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(PolicyError::NeedsDigit);
    }
    if policy.require_symbol && !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(PolicyError::NeedsSymbol);
    }
    for h in history {
        if verify_password(password, h) {
            return Err(PolicyError::Reused);
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum PasswordState {
    Valid,
    Expired,
    ForcedChange,
}

pub fn password_state(
    must_change: bool,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> PasswordState {
    if must_change {
        return PasswordState::ForcedChange;
    }
    match expires_at {
        Some(exp) if exp < chrono::Utc::now() => PasswordState::Expired,
        _ => PasswordState::Valid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pol() -> PasswordPolicy {
        PasswordPolicy {
            min_length: 8,
            require_upper: true,
            require_lower: true,
            require_digit: true,
            require_symbol: true,
            max_reuse: 3,
            days_to_expire: 90,
            block_common: true,
        }
    }

    #[test]
    fn too_short() {
        assert_eq!(validate_password(&pol(), "Ab1!", &[]), Err(PolicyError::TooShort));
    }

    #[test]
    fn missing_symbol() {
        assert_eq!(validate_password(&pol(), "Abcdefg1", &[]), Err(PolicyError::NeedsSymbol));
    }

    #[test]
    fn common_blocked() {
        assert_eq!(validate_password(&pol(), "password", &[]), Err(PolicyError::CommonPassword));
    }

    #[test]
    fn reused_blocked() {
        let h = hash_password("CorrectHorse1!");
        assert_eq!(validate_password(&pol(), "CorrectHorse1!", &[h]), Err(PolicyError::Reused));
    }

    #[test]
    fn valid_pass() {
        assert_eq!(validate_password(&pol(), "CorrectHorse1!", &[]), Ok(()));
    }

    #[test]
    fn state_transitions() {
        use chrono::Utc;
        assert_eq!(
            password_state(false, Some(Utc::now() - chrono::Duration::days(1))),
            PasswordState::Expired
        );
        assert_eq!(
            password_state(true, Some(Utc::now() + chrono::Duration::days(1))),
            PasswordState::ForcedChange
        );
        assert_eq!(password_state(false, None), PasswordState::Valid);
    }
}
