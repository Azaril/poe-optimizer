//! PoB lowercases modifier text for grammar matching, but not item formatting.
//! These borrowed projections preserve numeric spelling and all source bytes.
pub(crate) fn strip_prefix_ascii<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.get(..prefix.len())
        .filter(|actual| actual.eq_ignore_ascii_case(prefix))?;
    text.get(prefix.len()..)
}
pub(crate) fn split_once_ascii<'a>(text: &'a str, delimiter: &str) -> Option<(&'a str, &'a str)> {
    if delimiter.is_empty() {
        return Some(("", text));
    }
    let start = text
        .as_bytes()
        .windows(delimiter.len())
        .position(|part| part.eq_ignore_ascii_case(delimiter.as_bytes()))?;
    Some((text.get(..start)?, text.get(start + delimiter.len()..)?))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_ascii_matching_preserves_numeric_and_unicode_bytes() {
        assert_eq!(
            strip_prefix_ascii("ADDS 003 to 07 Physical Damage", "Adds "),
            Some("003 to 07 Physical Damage")
        );
        assert_eq!(
            split_once_ascii("+17.50 TO évasion RATING", " to évasion "),
            Some(("+17.50", "RATING"))
        );
        assert_eq!(strip_prefix_ascii("Évasion", "évasion"), None);
        assert_eq!(split_once_ascii("xé", "é"), Some(("x", "")));
        assert_eq!(split_once_ascii("a", "LONG"), None);
        assert_eq!(strip_prefix_ascii("x", "long"), None);
    }
}
