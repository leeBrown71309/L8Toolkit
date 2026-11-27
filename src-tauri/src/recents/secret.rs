//! Spots copied texts that look like a password, an API key or a token, so
//! Recents masks them even when the app that copied them does not say so.
//!
//! Desktop password managers are recognised by their process, but a browser
//! extension (Bitwarden or 1Password in Chrome, Edge, Vivaldi…) copies through
//! the browser, which marks nothing as confidential. The content is then the
//! only clue. The rules lean towards masking: a masked text still copies and
//! drags normally, while a secret shown in clear cannot be taken back.

/// Shorter texts are codes or words, not passwords worth hiding.
const MIN_CHARS: usize = 8;
/// Longer single "words" are data (a base64 image, a minified line), not secrets.
const MAX_CHARS: usize = 512;
/// Above this, a password-like mix is more likely a key than something typed.
const MAX_PASSWORD_CHARS: usize = 64;
/// A run of letters and digits this long reads as a key ("sk-…", "ghp_…", hex).
const MIN_TOKEN_RUN: usize = 16;

/// True when `text` looks like a secret: one "word" (no spaces) that mixes
/// kinds of characters like a generated password, or carries a long random
/// run of letters and digits like an API key. URLs, e-mail addresses, paths,
/// dates, numbers and version strings are left visible.
pub fn looks_like_secret(text: &str) -> bool {
    let value = text.trim();
    let length = value.chars().count();
    if !(MIN_CHARS..=MAX_CHARS).contains(&length) || value.chars().any(char::is_whitespace) {
        return false;
    }
    if is_address(value) || is_numeric_like(value) || is_version(value) {
        return false;
    }
    has_token_run(value) || (length <= MAX_PASSWORD_CHARS && character_kinds(value) >= 3)
}

/// URLs, e-mail addresses and file paths: copied all the time, never secret.
fn is_address(value: &str) -> bool {
    let lower = value.to_lowercase();
    if lower.contains("://") || lower.starts_with("www.") || lower.starts_with("mailto:") {
        return true;
    }
    let bytes = value.as_bytes();
    let drive_path = bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/');
    if drive_path || value.starts_with("\\\\") || value.starts_with('/') || value.starts_with("~/") || value.starts_with("./") {
        return true;
    }
    is_email(value)
}

fn is_email(value: &str) -> bool {
    let mut parts = value.split('@');
    let (Some(user), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !user.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && domain.chars().all(|character| character.is_alphanumeric() || matches!(character, '.' | '-'))
}

/// Dates, times, phone numbers, amounts: digits and separators only.
fn is_numeric_like(value: &str) -> bool {
    value.chars().all(|character| character.is_ascii_digit() || matches!(character, '.' | ',' | ':' | '-' | '/' | '+' | '(' | ')'))
}

/// "1.2.3", "v0.1.4", "2.0.0-beta.1".
fn is_version(value: &str) -> bool {
    let core = value.strip_prefix(['v', 'V']).unwrap_or(value);
    let (numbers, suffix) = core.split_once(['-', '+']).unwrap_or((core, ""));
    let parts: Vec<&str> = numbers.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|part| !part.is_empty() && part.chars().all(|character| character.is_ascii_digit()))
        && suffix.chars().all(|character| character.is_ascii_alphanumeric() || character == '.')
}

/// A long run of letters and digits with both in it: API keys and tokens.
fn has_token_run(value: &str) -> bool {
    value.split(|character: char| !character.is_ascii_alphanumeric()).any(|run| {
        run.len() >= MIN_TOKEN_RUN
            && run.chars().any(|character| character.is_ascii_alphabetic())
            && run.chars().any(|character| character.is_ascii_digit())
    })
}

/// How many of lowercase, uppercase, digits and symbols appear.
fn character_kinds(value: &str) -> usize {
    [
        value.chars().any(char::is_lowercase),
        value.chars().any(char::is_uppercase),
        value.chars().any(|character| character.is_ascii_digit()),
        value.chars().any(|character| !character.is_alphanumeric()),
    ]
    .into_iter()
    .filter(|present| *present)
    .count()
}

#[cfg(test)]
mod tests {
    use super::looks_like_secret;

    #[test]
    fn masks_generated_passwords() {
        for password in ["Tr0ub4dor&3", "Xk9!mP2#qL7$wR", "aB3dE6gH", "Bonjour2024!", "p@ssw0rd!"] {
            assert!(looks_like_secret(password), "{password}");
        }
    }

    #[test]
    fn masks_api_keys_and_tokens() {
        for key in [
            "sk-proj-4f9KqL2mZx8Wb1Nc7Rt5Yv3Hd6Jp0Se",
            "ghp_16C7e42F292c6912E7710c838347Ae178B4a",
            "a3f9c2e81b7d4f6a9e0c5b2d8f1a7e3c",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
            "  AKIAIOSFODNN7EXAMPLE  ",
        ] {
            assert!(looks_like_secret(key), "{key}");
        }
    }

    #[test]
    fn leaves_everyday_texts_visible() {
        for text in [
            "Bonjour à tous",
            "https://redcups.leeeight.site/",
            "www.example.com",
            "jean.dupont@gmail.com",
            r"C:\Users\Lee\Documents\Rapport annuel.pdf",
            "/usr/local/bin",
            "2026-11-27",
            "+33 6 12 34 56 78",
            "06.12.34.56.78",
            "v0.1.4",
            "2.0.0-beta.1",
            "internationalization",
            "feat(recents):",
            "snake_case_name",
            "123456",
            "Short1!",
        ] {
            assert!(!looks_like_secret(text), "{text}");
        }
    }

    #[test]
    fn leaves_long_data_visible() {
        let data = "A1".repeat(400);
        assert!(!looks_like_secret(&data));
    }
}
