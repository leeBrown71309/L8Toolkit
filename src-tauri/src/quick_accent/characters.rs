//! Accented characters offered for each letter, per language.
//!
//! Each language lists, for the letters it accents, its characters from the
//! most to the least used. Several languages combine in the order chosen in
//! the settings, without duplicates, so the first entry is the most likely
//! character for the user's main language. Two extra sets are not languages:
//! currencies and common symbols, each under a memorable letter.

use serde::{Deserialize, Serialize};

/// Character sets that can be offered. Codes are what the settings store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccentLanguage {
    Fr,
    Es,
    Pt,
    De,
    It,
    Nl,
    Ca,
    Tr,
    Pl,
    Ro,
    Cs,
    Sk,
    Hu,
    /// Croatian, Serbian (Latin) and Bosnian share their letters.
    Hr,
    Sl,
    /// Danish, Norwegian and Swedish share their letters.
    Nordic,
    Fi,
    Is,
    Et,
    Lv,
    Lt,
    Mt,
    Cy,
    /// Irish and Scottish Gaelic.
    Ga,
    Sq,
    Ku,
    Eo,
    Vi,
    Mi,
    Pinyin,
    Currency,
    Symbols,
}

impl AccentLanguage {
    /// Order used when every set is selected: the most spoken first, the
    /// non-language sets last.
    pub const ALL: [AccentLanguage; 32] = [
        AccentLanguage::Fr,
        AccentLanguage::Es,
        AccentLanguage::Pt,
        AccentLanguage::De,
        AccentLanguage::It,
        AccentLanguage::Nl,
        AccentLanguage::Ca,
        AccentLanguage::Tr,
        AccentLanguage::Pl,
        AccentLanguage::Ro,
        AccentLanguage::Cs,
        AccentLanguage::Sk,
        AccentLanguage::Hu,
        AccentLanguage::Hr,
        AccentLanguage::Sl,
        AccentLanguage::Nordic,
        AccentLanguage::Fi,
        AccentLanguage::Is,
        AccentLanguage::Et,
        AccentLanguage::Lv,
        AccentLanguage::Lt,
        AccentLanguage::Mt,
        AccentLanguage::Cy,
        AccentLanguage::Ga,
        AccentLanguage::Sq,
        AccentLanguage::Ku,
        AccentLanguage::Eo,
        AccentLanguage::Vi,
        AccentLanguage::Mi,
        AccentLanguage::Pinyin,
        AccentLanguage::Currency,
        AccentLanguage::Symbols,
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
            0x03 => AccentLanguage::Ca,
            0x1F => AccentLanguage::Tr,
            0x15 => AccentLanguage::Pl,
            0x18 => AccentLanguage::Ro,
            0x05 => AccentLanguage::Cs,
            0x1B => AccentLanguage::Sk,
            0x0E => AccentLanguage::Hu,
            0x1A => AccentLanguage::Hr,
            0x24 => AccentLanguage::Sl,
            0x06 | 0x14 | 0x1D => AccentLanguage::Nordic,
            0x0B => AccentLanguage::Fi,
            0x0F => AccentLanguage::Is,
            0x25 => AccentLanguage::Et,
            0x26 => AccentLanguage::Lv,
            0x27 => AccentLanguage::Lt,
            0x3A => AccentLanguage::Mt,
            0x52 => AccentLanguage::Cy,
            0x3C | 0x91 => AccentLanguage::Ga,
            0x1C => AccentLanguage::Sq,
            0x2A => AccentLanguage::Vi,
            0x81 => AccentLanguage::Mi,
            _ => return None,
        })
    }

    /// Lowercase characters for `letter` (lowercase a-z).
    fn accents(self, letter: char) -> &'static [char] {
        use AccentLanguage::*;
        match (self, letter) {
            (Fr, 'a') => &['à', 'â', 'æ', 'ä'],
            (Fr, 'c') => &['ç'],
            (Fr, 'e') => &['é', 'è', 'ê', 'ë', '€'],
            (Fr, 'i') => &['î', 'ï'],
            (Fr, 'o') => &['ô', 'œ', 'ö'],
            (Fr, 'u') => &['ù', 'û', 'ü'],
            (Fr, 'y') => &['ÿ'],

            (Es, 'a') => &['á', 'ª'],
            (Es, 'e') => &['é', '€'],
            (Es, 'i') => &['í'],
            (Es, 'n') => &['ñ'],
            (Es, 'o') => &['ó', 'º'],
            (Es, 'u') => &['ú', 'ü'],

            (Pt, 'a') => &['ã', 'á', 'â', 'à', 'ª'],
            (Pt, 'c') => &['ç'],
            (Pt, 'e') => &['é', 'ê', '€'],
            (Pt, 'i') => &['í'],
            (Pt, 'o') => &['õ', 'ó', 'ô', 'º'],
            (Pt, 'u') => &['ú', 'ü'],

            (De, 'a') => &['ä'],
            (De, 'e') => &['€'],
            (De, 'o') => &['ö'],
            (De, 's') => &['ß'],
            (De, 'u') => &['ü'],

            (It, 'a') => &['à'],
            (It, 'e') => &['è', 'é', '€'],
            (It, 'i') => &['ì', 'í'],
            (It, 'o') => &['ò', 'ó'],
            (It, 'u') => &['ù', 'ú'],

            (Nl, 'a') => &['á'],
            (Nl, 'e') => &['é', 'ë', 'è', 'ê', '€'],
            (Nl, 'i') => &['ï', 'í'],
            (Nl, 'o') => &['ö', 'ó'],
            (Nl, 'u') => &['ü', 'ú'],

            (Ca, 'a') => &['à'],
            (Ca, 'c') => &['ç'],
            (Ca, 'e') => &['è', 'é'],
            (Ca, 'i') => &['í', 'ï'],
            (Ca, 'l') => &['ŀ'],
            (Ca, 'o') => &['ò', 'ó'],
            (Ca, 'u') => &['ú', 'ü'],

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

            (Sk, 'a') => &['á', 'ä'],
            (Sk, 'c') => &['č'],
            (Sk, 'd') => &['ď'],
            (Sk, 'e') => &['é'],
            (Sk, 'i') => &['í'],
            (Sk, 'l') => &['ľ', 'ĺ'],
            (Sk, 'n') => &['ň'],
            (Sk, 'o') => &['ó', 'ô'],
            (Sk, 'r') => &['ŕ'],
            (Sk, 's') => &['š'],
            (Sk, 't') => &['ť'],
            (Sk, 'u') => &['ú'],
            (Sk, 'y') => &['ý'],
            (Sk, 'z') => &['ž'],

            (Hu, 'a') => &['á'],
            (Hu, 'e') => &['é'],
            (Hu, 'i') => &['í'],
            (Hu, 'o') => &['ó', 'ö', 'ő'],
            (Hu, 'u') => &['ú', 'ü', 'ű'],

            (Hr, 'c') => &['č', 'ć'],
            (Hr, 'd') => &['đ'],
            (Hr, 's') => &['š'],
            (Hr, 'z') => &['ž'],

            (Sl, 'c') => &['č'],
            (Sl, 's') => &['š'],
            (Sl, 'z') => &['ž'],

            (Nordic, 'a') => &['å', 'ä', 'æ'],
            (Nordic, 'o') => &['ø', 'ö'],

            (Fi, 'a') => &['ä', 'å'],
            (Fi, 'o') => &['ö'],
            (Fi, 's') => &['š'],
            (Fi, 'z') => &['ž'],

            (Is, 'a') => &['á', 'æ'],
            (Is, 'd') => &['ð'],
            (Is, 'e') => &['é'],
            (Is, 'i') => &['í'],
            (Is, 'o') => &['ó', 'ö'],
            (Is, 't') => &['þ'],
            (Is, 'u') => &['ú'],
            (Is, 'y') => &['ý'],

            (Et, 'a') => &['ä'],
            (Et, 'o') => &['õ', 'ö'],
            (Et, 's') => &['š'],
            (Et, 'u') => &['ü'],
            (Et, 'z') => &['ž'],

            (Lv, 'a') => &['ā'],
            (Lv, 'c') => &['č'],
            (Lv, 'e') => &['ē'],
            (Lv, 'g') => &['ģ'],
            (Lv, 'i') => &['ī'],
            (Lv, 'k') => &['ķ'],
            (Lv, 'l') => &['ļ'],
            (Lv, 'n') => &['ņ'],
            (Lv, 's') => &['š'],
            (Lv, 'u') => &['ū'],
            (Lv, 'z') => &['ž'],

            (Lt, 'a') => &['ą'],
            (Lt, 'c') => &['č'],
            (Lt, 'e') => &['ę', 'ė'],
            (Lt, 'i') => &['į'],
            (Lt, 's') => &['š'],
            (Lt, 'u') => &['ų', 'ū'],
            (Lt, 'z') => &['ž'],

            (Mt, 'a') => &['à'],
            (Mt, 'c') => &['ċ'],
            (Mt, 'e') => &['è'],
            (Mt, 'g') => &['ġ'],
            (Mt, 'h') => &['ħ'],
            (Mt, 'i') => &['ì', 'î'],
            (Mt, 'o') => &['ò'],
            (Mt, 'u') => &['ù'],
            (Mt, 'z') => &['ż'],

            (Cy, 'a') => &['â', 'ä', 'á', 'à'],
            (Cy, 'e') => &['ê', 'ë', 'é', 'è'],
            (Cy, 'i') => &['î', 'ï', 'í', 'ì'],
            (Cy, 'o') => &['ô', 'ö', 'ó', 'ò'],
            (Cy, 'u') => &['û', 'ü', 'ú', 'ù'],
            (Cy, 'w') => &['ŵ', 'ẅ', 'ẃ', 'ẁ'],
            (Cy, 'y') => &['ŷ', 'ÿ', 'ý', 'ỳ'],

            (Ga, 'a') => &['á', 'à'],
            (Ga, 'e') => &['é', 'è'],
            (Ga, 'i') => &['í', 'ì'],
            (Ga, 'o') => &['ó', 'ò'],
            (Ga, 'u') => &['ú', 'ù'],

            (Sq, 'c') => &['ç'],
            (Sq, 'e') => &['ë'],

            (Ku, 'c') => &['ç'],
            (Ku, 'e') => &['ê'],
            (Ku, 'i') => &['î'],
            (Ku, 's') => &['ş'],
            (Ku, 'u') => &['û'],

            (Eo, 'c') => &['ĉ'],
            (Eo, 'g') => &['ĝ'],
            (Eo, 'h') => &['ĥ'],
            (Eo, 'j') => &['ĵ'],
            (Eo, 's') => &['ŝ'],
            (Eo, 'u') => &['ŭ'],

            (Vi, 'a') => &[
                'à', 'á', 'ả', 'ã', 'ạ', 'ă', 'ằ', 'ắ', 'ẳ', 'ẵ', 'ặ', 'â', 'ầ', 'ấ', 'ẩ', 'ẫ', 'ậ',
            ],
            (Vi, 'd') => &['đ'],
            (Vi, 'e') => &['è', 'é', 'ẻ', 'ẽ', 'ẹ', 'ê', 'ề', 'ế', 'ể', 'ễ', 'ệ'],
            (Vi, 'i') => &['ì', 'í', 'ỉ', 'ĩ', 'ị'],
            (Vi, 'o') => &[
                'ò', 'ó', 'ỏ', 'õ', 'ọ', 'ô', 'ồ', 'ố', 'ổ', 'ỗ', 'ộ', 'ơ', 'ờ', 'ớ', 'ở', 'ỡ', 'ợ',
            ],
            (Vi, 'u') => &['ù', 'ú', 'ủ', 'ũ', 'ụ', 'ư', 'ừ', 'ứ', 'ử', 'ữ', 'ự'],
            (Vi, 'y') => &['ỳ', 'ý', 'ỷ', 'ỹ', 'ỵ'],

            (Mi, 'a') => &['ā'],
            (Mi, 'e') => &['ē'],
            (Mi, 'i') => &['ī'],
            (Mi, 'o') => &['ō'],
            (Mi, 'u') => &['ū'],

            (Pinyin, 'a') => &['ā', 'á', 'ǎ', 'à'],
            (Pinyin, 'e') => &['ē', 'é', 'ě', 'è'],
            (Pinyin, 'i') => &['ī', 'í', 'ǐ', 'ì'],
            (Pinyin, 'o') => &['ō', 'ó', 'ǒ', 'ò'],
            (Pinyin, 'u') => &['ū', 'ú', 'ǔ', 'ù', 'ü', 'ǖ', 'ǘ', 'ǚ', 'ǜ'],
            (Pinyin, 'v') => &['ü', 'ǖ', 'ǘ', 'ǚ', 'ǜ'],

            (Currency, 'b') => &['₿', '฿'],
            (Currency, 'c') => &['¢', '₵'],
            (Currency, 'd') => &['$', '₫'],
            (Currency, 'e') => &['€'],
            (Currency, 'f') => &['₣', 'ƒ'],
            (Currency, 'g') => &['₲'],
            (Currency, 'h') => &['₴'],
            (Currency, 'k') => &['₭', '₸'],
            (Currency, 'l') => &['£', '₺', '₾'],
            (Currency, 'm') => &['₼'],
            (Currency, 'n') => &['₦'],
            (Currency, 'p') => &['₱', '₽'],
            (Currency, 'r') => &['₹', '₽', '₨'],
            (Currency, 's') => &['$', '₪'],
            (Currency, 't') => &['₺', '₮'],
            (Currency, 'w') => &['₩'],
            (Currency, 'y') => &['¥'],

            (Symbols, 'a') => &['@', 'ª'],
            (Symbols, 'c') => &['©'],
            (Symbols, 'd') => &['°', '†', '‡'],
            (Symbols, 'e') => &['…', '–', '—'],
            (Symbols, 'm') => &['µ'],
            (Symbols, 'n') => &['№'],
            (Symbols, 'o') => &['°', 'º'],
            (Symbols, 'p') => &['¶', 'π', '‰'],
            (Symbols, 'q') => &['«', '»', '“', '”', '‘', '’'],
            (Symbols, 'r') => &['®'],
            (Symbols, 's') => &['§'],
            (Symbols, 't') => &['™', '†'],
            (Symbols, 'x') => &['×', '÷', '±'],

            _ => &[],
        }
    }
}

/// Characters offered for `letter`, in display order. An empty `languages`
/// list means every set.
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
/// on the same key. Symbols have no case and stay as they are.
fn to_upper(character: char) -> char {
    match character {
        'ß' => 'ẞ',
        'ı' => 'İ',
        other => {
            let mut upper = other.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(single), None) => single,
                _ => other,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_letters_list_the_common_accents_first() {
        assert_eq!(characters_for('e', &[AccentLanguage::Fr], false), vec!['é', 'è', 'ê', 'ë', '€']);
        assert_eq!(characters_for('c', &[AccentLanguage::Fr], false), vec!['ç']);
        assert!(characters_for('k', &[AccentLanguage::Fr], false).is_empty());
    }

    #[test]
    fn languages_combine_in_the_chosen_order_without_duplicates() {
        let combined = characters_for('e', &[AccentLanguage::Es, AccentLanguage::Fr], false);
        assert_eq!(combined, vec!['é', '€', 'è', 'ê', 'ë']);
        let all = characters_for('e', &[], false);
        assert_eq!(all.first(), Some(&'é'));
        assert!(all.contains(&'ę') && all.contains(&'ě') && all.contains(&'ệ'));
        let unique: std::collections::HashSet<_> = all.iter().collect();
        assert_eq!(unique.len(), all.len());
    }

    #[test]
    fn uppercase_follows_the_typing_conventions() {
        assert_eq!(characters_for('a', &[AccentLanguage::Fr], true), vec!['À', 'Â', 'Æ', 'Ä']);
        assert_eq!(characters_for('s', &[AccentLanguage::De], true), vec!['ẞ']);
        assert_eq!(characters_for('i', &[AccentLanguage::Tr], true), vec!['İ', 'Î']);
        assert_eq!(characters_for('o', &[AccentLanguage::Fr], true), vec!['Ô', 'Œ', 'Ö']);
        // Symbols have no case.
        assert_eq!(characters_for('e', &[AccentLanguage::Currency], true), vec!['€']);
        assert_eq!(characters_for('a', &[AccentLanguage::Symbols], true), vec!['@', 'ª']);
    }

    #[test]
    fn windows_languages_map_to_their_accents() {
        assert_eq!(AccentLanguage::from_windows_primary_language(0x0C), Some(AccentLanguage::Fr));
        assert_eq!(AccentLanguage::from_windows_primary_language(0x1D), Some(AccentLanguage::Nordic));
        assert_eq!(AccentLanguage::from_windows_primary_language(0x2A), Some(AccentLanguage::Vi));
        assert_eq!(AccentLanguage::from_windows_primary_language(0x09), None);
    }

    #[test]
    fn every_set_offers_something_and_never_plain_letters() {
        for language in AccentLanguage::ALL {
            let mut offered = 0;
            for letter in 'a'..='z' {
                let characters = characters_for(letter, &[language], false);
                assert!(characters.iter().all(|character| !character.is_ascii_alphabetic()), "{language:?} {letter}");
                offered += characters.len();
            }
            assert!(offered > 0, "{language:?} offers nothing");
        }
    }

    #[test]
    fn all_sets_together_offer_far_more_than_one_language() {
        let one: usize = ('a'..='z').map(|letter| characters_for(letter, &[AccentLanguage::Fr], false).len()).sum();
        let all: usize = ('a'..='z').map(|letter| characters_for(letter, &[], false).len()).sum();
        assert!(all > one * 5, "{all} characters in all, {one} in French");
    }
}
