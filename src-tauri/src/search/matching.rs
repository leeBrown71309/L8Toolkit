//! Fuzzy matching (nucleo, the engine behind the Helix editor picker) plus the
//! launcher-specific bonuses that make "vsc" find Visual Studio Code first.

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

const EXACT_NAME_BONUS: u32 = 220;
const PREFIX_BONUS: u32 = 130;
const ACRONYM_BONUS: u32 = 90;
const WORD_PREFIX_BONUS: u32 = 60;

pub struct FuzzyQuery {
    pattern: Pattern,
    matcher: Matcher,
    buffer: Vec<char>,
    pub lowercase: String,
}

impl FuzzyQuery {
    pub fn new(query: &str) -> Self {
        Self {
            // `AtomKind::Fuzzy` disables fzf syntax (^, $, !) so symbols in a
            // query such as "c++" are matched literally.
            pattern: Pattern::new(query, CaseMatching::Ignore, Normalization::Smart, AtomKind::Fuzzy),
            matcher: Matcher::new(Config::DEFAULT),
            buffer: Vec::new(),
            lowercase: query.to_lowercase(),
        }
    }

    pub fn score(&mut self, haystack: Utf32Str<'_>) -> Option<u32> {
        self.pattern.score(haystack, &mut self.matcher)
    }

    pub fn score_text(&mut self, text: &str) -> Option<u32> {
        let haystack = Utf32Str::new(text, &mut self.buffer);
        self.pattern.score(haystack, &mut self.matcher)
    }

    /// Sorted, de-duplicated character indices of the match inside `text`.
    pub fn indices(&mut self, text: &str) -> Vec<u32> {
        let mut indices = Vec::new();
        let haystack = Utf32Str::new(text, &mut self.buffer);
        self.pattern.indices(haystack, &mut self.matcher, &mut indices);
        indices.sort_unstable();
        indices.dedup();
        indices
    }
}

/// First letter of every word: "Visual Studio Code" -> "vsc".
pub fn initials_of(name: &str) -> String {
    name.split(|character: char| !character.is_alphanumeric())
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Extra points for matches a human would rank first.
pub fn name_bonus(name_lowercase: &str, initials: &str, query_lowercase: &str) -> u32 {
    if query_lowercase.is_empty() {
        return 0;
    }
    if name_lowercase == query_lowercase {
        return EXACT_NAME_BONUS;
    }
    if name_lowercase.starts_with(query_lowercase) {
        return PREFIX_BONUS;
    }
    if query_lowercase.chars().count() >= 2 && initials.starts_with(query_lowercase) {
        return ACRONYM_BONUS;
    }
    let starts_a_word = name_lowercase
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| word.starts_with(query_lowercase));
    if starts_a_word {
        return WORD_PREFIX_BONUS;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_skip_separators() {
        assert_eq!(initials_of("Visual Studio Code"), "vsc");
        assert_eq!(initials_of("Microsoft Edge (Beta)"), "meb");
    }

    #[test]
    fn bonus_prefers_exact_then_prefix_then_acronym() {
        let exact = name_bonus("code", "c", "code");
        let prefix = name_bonus("code insiders", "ci", "code");
        let acronym = name_bonus("visual studio code", "vsc", "vsc");
        let word = name_bonus("visual studio code", "vsc", "studio");
        assert!(exact > prefix && prefix > acronym && acronym > word && word > 0);
        assert_eq!(name_bonus("notepad", "n", "xyz"), 0);
    }

    #[test]
    fn indices_point_at_matched_characters() {
        let mut query = FuzzyQuery::new("vsc");
        assert_eq!(query.indices("Visual Studio Code"), vec![0, 7, 14]);
        assert!(query.score_text("Notepad").is_none());
    }
}
