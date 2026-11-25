use windows::Win32::Globalization::GetUserDefaultUILanguage;

/// Low 10 bits of a LANGID: the primary language (0x09 English, 0x0C French...).
const PRIMARY_LANGUAGE_MASK: u16 = 0x03FF;

/// Primary language of the Windows display language.
pub fn system_primary_language() -> Option<u16> {
    // SAFETY: read-only query without arguments.
    let language_id = unsafe { GetUserDefaultUILanguage() };
    (language_id != 0).then_some(language_id & PRIMARY_LANGUAGE_MASK)
}
