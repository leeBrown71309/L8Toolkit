//! Web helpers: URL detection and search-engine URLs.

use crate::settings::WebSearchEngine;

/// Top-level domains accepted without an explicit scheme. Kept short on purpose
/// so file names like "report.txt" are not mistaken for websites.
const KNOWN_TOP_LEVEL_DOMAINS: &[&str] = &[
    "com", "org", "net", "io", "dev", "app", "ai", "co", "fr", "be", "ch", "ca", "uk", "de", "es",
    "it", "eu", "gov", "edu", "me", "tv", "info", "xyz", "so", "gg", "sh", "tech", "cd", "cg",
];

/// Returns a navigable URL when the query looks like one.
pub fn normalize_url(input: &str) -> Option<String> {
    let candidate = input.trim();
    if candidate.is_empty() || candidate.contains(char::is_whitespace) || candidate.contains('\\') {
        return None;
    }
    let lowercase = candidate.to_lowercase();
    if lowercase.starts_with("http://") || lowercase.starts_with("https://") {
        return Some(candidate.to_string());
    }
    if lowercase.starts_with("www.") {
        return Some(format!("https://{candidate}"));
    }
    if lowercase.starts_with("localhost:") {
        return Some(format!("http://{candidate}"));
    }

    let host = lowercase.split(['/', '?', '#']).next().unwrap_or_default();
    let host_without_port = host.split(':').next().unwrap_or_default();
    let (name, top_level_domain) = host_without_port.rsplit_once('.')?;
    let valid_name = !name.is_empty()
        && name.chars().all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '.');
    (valid_name && KNOWN_TOP_LEVEL_DOMAINS.contains(&top_level_domain))
        .then(|| format!("https://{candidate}"))
}

pub fn engine_label(engine: WebSearchEngine) -> &'static str {
    match engine {
        WebSearchEngine::Google => "Google",
        WebSearchEngine::Bing => "Bing",
        WebSearchEngine::DuckDuckGo => "DuckDuckGo",
        WebSearchEngine::Brave => "Brave Search",
    }
}

pub fn search_url(engine: WebSearchEngine, query: &str) -> String {
    let base = match engine {
        WebSearchEngine::Google => "https://www.google.com/search?q=",
        WebSearchEngine::Bing => "https://www.bing.com/search?q=",
        WebSearchEngine::DuckDuckGo => "https://duckduckgo.com/?q=",
        WebSearchEngine::Brave => "https://search.brave.com/search?q=",
    };
    format!("{base}{}", percent_encode(query.trim()))
}

/// RFC 3986 encoding of everything except unreserved characters.
fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_urls() {
        assert_eq!(normalize_url("github.com"), Some("https://github.com".into()));
        assert_eq!(normalize_url("tauri.app/start"), Some("https://tauri.app/start".into()));
        assert_eq!(normalize_url("https://a.b/c"), Some("https://a.b/c".into()));
        assert_eq!(normalize_url("localhost:1420"), Some("http://localhost:1420".into()));
    }

    #[test]
    fn rejects_files_and_sentences() {
        assert_eq!(normalize_url("report.txt"), None);
        // Source files must stay file searches, not websites.
        assert_eq!(normalize_url("main.rs"), None);
        assert_eq!(normalize_url("hello world.com"), None);
        assert_eq!(normalize_url("C:\\Users"), None);
        assert_eq!(normalize_url("notepad"), None);
    }

    #[test]
    fn encodes_queries() {
        assert_eq!(
            search_url(WebSearchEngine::Google, "rust été & co"),
            "https://www.google.com/search?q=rust%20%C3%A9t%C3%A9%20%26%20co"
        );
    }
}
