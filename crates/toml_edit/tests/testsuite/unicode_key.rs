use toml_edit::DocumentMut;

/// "é" as a single precomposed code point (NFC).
const NFC: &str = "caf\u{e9}";
/// "é" as `e` followed by a combining acute accent (NFD).
const NFD: &str = "cafe\u{301}";

#[test]
fn nfd_key_is_canonicalized_while_preserving_the_raw_spelling() {
    let input = format!("\"{NFD}\" = 2\n");
    let document = input.parse::<DocumentMut>().unwrap();

    // The encoded document keeps its original representation.
    assert_eq!(document.to_string(), input);
    // Lookups use the canonical key identity.
    assert_eq!(document[NFC].as_integer(), Some(2));
}

#[test]
fn canonically_equivalent_keys_are_duplicates() {
    let input = format!("\"{NFC}\" = 1\n\"{NFD}\" = 2\n");
    let error = input.parse::<DocumentMut>().unwrap_err();
    assert_eq!(error.message(), "duplicate key");
}

#[test]
fn canonically_equivalent_table_headers_are_duplicates() {
    let input = format!("[\"{NFC}\"]\nx = 1\n[\"{NFD}\"]\ny = 2\n");
    let error = input.parse::<DocumentMut>().unwrap_err();
    assert_eq!(error.message(), "duplicate key");
}

#[test]
fn distinct_unicode_keys_remain_distinct() {
    let input = "\"caf\u{e9}\" = 1\n\"na\u{ef}ve\" = 2\n\"\u{65e5}\u{672c}\" = 3\n";
    let document = input.parse::<DocumentMut>().unwrap();
    assert_eq!(document.as_table().len(), 3);
    assert_eq!(document["caf\u{e9}"].as_integer(), Some(1));
    assert_eq!(document["na\u{ef}ve"].as_integer(), Some(2));
    assert_eq!(document["\u{65e5}\u{672c}"].as_integer(), Some(3));
}
