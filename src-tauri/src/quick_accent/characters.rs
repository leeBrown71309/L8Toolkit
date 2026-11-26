//! Accented characters offered for each letter, per language.
//!
//! Each language lists, for the letters it accents, its characters from the
//! most to the least used. Several languages combine in the order chosen in
//! the settings, without duplicates, so the first entry is the most likely
//! character for the user's main language.

use serde::{Deserialize, Serialize};

/// Languages whose accents can be offered. Codes are what the settings store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccentLanguage {
    Fr,
    Es,
    Pt,
    De,
    It,
    Nl,
    Tr,
    Pl,
    Ro,
    Cs,
    /// Danish, Norwegian and Swedish share their letters.
    Nordic,
}

impl AccentLanguage {
    /// Order used when every language is selected.
    pub const ALL: [AccentLanguage; 11] = [
        AccentLanguage::Fr,
        AccentLanguage::Es,
        AccentLanguage::Pt,
        AccentLanguage::De,
        AccentLanguage::It,
        AccentLanguage::Nl,
        AccentLanguage::Tr,
        AccentLanguage::Pl,
        AccentLanguage::Ro,
        AccentLanguage::Cs,
        AccentLanguage::Nordic,
    ];

    /// The language matching a Windows primary language id, if supported.
    pub fn from_windows_primary_language(primary: u16) -> Option<Self> {
        Some(match primary {
            0x0C => AccentLanguage::Fr,
            0x0A => AccentLanguage::Es,
            0x16 => AccentLanguage::Pt,
            0x07 => AccentLanguage::De,
            0x10 => AccentLanguage::It,
            0x13 => AccentLanguage::Nl,
            0x1F => AccentLanguage::Tr,
            0x15 => AccentLanguage::Pl,
            0x18 => AccentLanguage::Ro,
            0x05 => AccentLanguage::Cs,
            0x06 | 0x14 | 0x1D => AccentLanguage::Nordic,
            _ => return None,
        })
    }

    /// Lowercase accented characters for `letter` (lowercase a-z).
    fn accents(self, letter: char) -> &'static [char] {
        use AccentLanguage::*;
        match (self, letter) {
            (Fr, 'a') => &['à', 'â', 'æ', 'ä'],
            (Fr, 'c') => &['ç'],
            (Fr, 'e') => &['é', 'è', 'ê', 'ë'],
            (Fr, 'i') => &['î', 'ï'],
            (Fr, 'o') => &['ô', 'œ', 'ö'],
            (Fr, 'u') => &['ù', 'û', 'ü'],
            (Fr, 'y') => &['ÿ'],

            (Es, 'a') => &['á'],
            (Es, 'e') => &['é'],
            (Es, 'i') => &['í'],
            (Es, 'n') => &['ñ'],
            (Es, 'o') => &['ó'],
            (Es, 'u') => &['ú', 'ü'],

            (Pt, 'a') => &['ã', 'á', 'â', 'à'],
            (Pt, 'c') => &['ç'],
            (Pt, 'e') => &['é', 'ê'],
            (Pt, 'i') => &['í'],
            (Pt, 'o') => &['õ', 'ó', 'ô'],
            (Pt, 'u') => &['ú', 'ü'],

            (De, 'a') => &['ä'],
            (De, 'o') => &['ö'],
            (De, 's') => &['ß'],
            (De, 'u') => &['ü'],

            (It, 'a') => &['à'],
            (It, 'e') => &['è', 'é'],
            (It, 'i') => &['ì', 'í'],
            (It, 'o') => &['ò', 'ó'],
            (It, 'u') => &['ù', 'ú'],

            (Nl, 'a') => &['á'],
            (Nl, 'e') => &['é', 'ë', 'è', 'ê'],
            (Nl, 'i') => &['ï', 'í'],
            (Nl, 'o') => &['ö', 'ó'],
            (Nl, 'u') => &['ü', 'ú'],

            (Tr, 'a') => &['â'],
            (Tr, 'c') => &['ç'],
            (Tr, 'g') => &['ğ'],
            (Tr, 'i') => &['ı', 'î'],
            (Tr, 'o') => &['ö'],
            (Tr, 's') => &['ş'],
            (Tr, 'u') => &['ü', 'û'],

            (Pl, 'a') => &['ą'],
            (Pl, 'c') => &['ć'],
            (Pl, 'e') => &['ę'],
            (Pl, 'l') => &['ł'],
            (Pl, 'n') => &['ń'],
            (Pl, 'o') => &['ó'],
            (Pl, 's') => &['ś'],
            (Pl, 'z') => &['ż', 'ź'],

            (Ro, 'a') => &['ă', 'â'],
            (Ro, 'i') => &['î'],
            (Ro, 's') => &['ș'],
            (Ro, 't') => &['ț'],

            (Cs, 'a') => &['á'],
            (Cs, 'c') => &['č'],
            (Cs, 'd') => &['ď'],
            (Cs, 'e') => &['é', 'ě'],
            (Cs, 'i') => &['í'],
            (Cs, 'n') => &['ň'],
            (Cs, 'o') => &['ó'],
            (Cs, 'r') => &['ř'],
            (Cs, 's') => &['š'],
            (Cs, 't') => &['ť'],
            (Cs, 'u') => &['ú', 'ů'],
            (Cs, 'y') => &['ý'],
            (Cs, 'z') => &['ž'],

            (Nordic, 'a') => &['å', 'ä', 'æ'],
            (Nordic, 'o') => &['ø', 'ö'],

            _ => &[],
        }
    }
}

/// Characters offered for `letter`, in display order. An empty `languages`
/// list means every language.
pub fn characters_for(letter: char, languages: &[AccentLanguage], uppercase: bool) -> Vec<char> {
    let languages: &[AccentLanguage] = if languages.is_empty() { &AccentLanguage::ALL } else { languages };
    let letter = letter.to_ascii_lowercase();
    let mut characters: Vec<char> = Vec::new();
    for language in languages {
        for &accent in language.accents(letter) {
            let shown = if uppercase { to_upper(accent) } else { accent };
            if !characters.contains(&shown) {
                characters.push(shown);
            }
        }
    }
    characters
}

/// Uppercase forms, including the two that `char::to_uppercase` gets wrong
/// for typing: ß has a capital (ẞ), and the Turkish dotless ı pairs with İ
/// on the same key.
fn to_upper(character: char) -> char {
    match character {
        'ß' => 'ẞ',
        'ı' => 'İ',
        other => other.to_uppercase().next().unwrap_or(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_letters_list_the_common_accents_first() {
        assert_eq!(characters_for('e', &[AccentLanguage::Fr], false), vec!['é', 'è', 'ê', 'ë']);
        assert_eq!(characters_for('c', &[AccentLanguage::Fr], false), vec!['ç']);
        assert!(characters_for('k', &[AccentLanguage::Fr], false).is_empty());
    }

    #[test]
    fn languages_combine_in_the_chosen_order_without_duplicates() {
        let combined = characters_for('e', &[AccentLanguage::Es, AccentLanguage::Fr], false);
        assert_eq!(combined, vec!['é', 'è', 'ê', 'ë']);
        let all = characters_for('e', &[], false);
        assert_eq!(all.first(), Some(&'é'));
        assert!(all.contains(&'ę') && all.contains(&'ě'));
        let unique: std::collections::HashSet<_> = all.iter().collect();
        assert_eq!(unique.len(), all.len());
    }

    #[test]
    fn uppercase_follows_the_typing_conventions() {
        assert_eq!(characters_for('a', &[AccentLanguage::Fr], true), vec!['À', 'Â', 'Æ', 'Ä']);
        assert_eq!(characters_for('s', &[AccentLanguage::De], true), vec!['ẞ']);
        assert_eq!(characters_for('i', &[AccentLanguage::Tr], true), vec!['İ', 'Î']);
        assert_eq!(characters_for('o', &[AccentLanguage::Fr], true), vec!['Ô', 'Œ', 'Ö']);
    }

    #[test]
    fn windows_languages_map_to_their_accents() {
        assert_eq!(AccentLanguage::from_windows_primary_language(0x0C), Some(AccentLanguage::Fr));
        assert_eq!(AccentLanguage::from_windows_primary_language(0x1D), Some(AccentLanguage::Nordic));
        assert_eq!(AccentLanguage::from_windows_primary_language(0x09), None);
    }

    #[test]
    fn every_language_offers_something() {
        for language in AccentLanguage::ALL {
            let offered = ('a'..='z').filter(|letter| !characters_for(*letter, &[language], false).is_empty()).count();
            assert!(offered > 0, "{language:?} offers no accent");
        }
    }
}
