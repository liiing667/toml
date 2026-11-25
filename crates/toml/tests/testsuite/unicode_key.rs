use std::collections::BTreeMap;

use toml::Value;

/// "é" as a single precomposed code point (NFC).
const NFC: &str = "caf\u{e9}";
/// "é" as `e` followed by a combining acute accent (NFD).
const NFD: &str = "cafe\u{301}";

#[test]
fn nfd_key_is_canonicalized_to_nfc() {
    let document = format!("\"{NFD}\" = 2\n");
    let value: Value = toml::from_str(&document).unwrap();
    let table = value.as_table().unwrap();

    assert_eq!(table.len(), 1);
    assert!(table.contains_key(NFC));
    assert_eq!(table[NFC].as_integer(), Some(2));
}

#[test]
fn canonically_equivalent_keys_are_duplicates() {
    let document = format!("\"{NFC}\" = 1\n\"{NFD}\" = 2\n");
    let error = toml::from_str::<Value>(&document).unwrap_err();
    assert_eq!(error.message(), "duplicate key");
}

#[test]
fn canonically_equivalent_dotted_keys_are_duplicates() {
    let document = format!("\"{NFC}\".a = 1\n\"{NFD}\".a = 2\n");
    let error = toml::from_str::<Value>(&document).unwrap_err();
    assert_eq!(error.message(), "duplicate key");
}

#[test]
fn canonically_equivalent_table_headers_are_duplicates() {
    let document = format!("[\"{NFC}\"]\nx = 1\n[\"{NFD}\"]\ny = 2\n");
    let error = toml::from_str::<Value>(&document).unwrap_err();
    assert_eq!(error.message(), "duplicate key");
}

#[test]
fn canonically_equivalent_inline_table_keys_are_duplicates() {
    let document = format!("t = {{ \"{NFC}\" = 1, \"{NFD}\" = 2 }}\n");
    assert!(toml::from_str::<Value>(&document).is_err());
}

#[test]
fn distinct_unicode_keys_remain_distinct() {
    let document = "\"caf\u{e9}\" = 1\n\"na\u{ef}ve\" = 2\n\"\u{65e5}\u{672c}\" = 3\n";
    let value: Value = toml::from_str(document).unwrap();
    let table = value.as_table().unwrap();
    assert_eq!(table.len(), 3);
    assert_eq!(table["caf\u{e9}"].as_integer(), Some(1));
    assert_eq!(table["na\u{ef}ve"].as_integer(), Some(2));
    assert_eq!(table["\u{65e5}\u{672c}"].as_integer(), Some(3));
}

#[test]
fn escaped_nfd_key_is_canonicalized_to_nfc() {
    let document = "\"x\\u0301\" = 7\n";
    let value: Value = toml::from_str(document).unwrap();
    let table = value.as_table().unwrap();
    assert_eq!(table.len(), 1);
    assert_eq!(table["x\u{301}"].as_integer(), Some(7));
}

#[test]
fn canonical_key_works_with_serde_field_names() {
    let document = format!("\"{NFD}\" = 2\n");
    let map = toml::from_str::<BTreeMap<String, i64>>(&document).unwrap();
    assert_eq!(map.get(NFC), Some(&2));
    assert_eq!(map.len(), 1);
}

#[test]
fn ascii_keys_keep_working() {
    let value: Value = toml::from_str("key = 1\nother = \"two\"\n").unwrap();
    let table = value.as_table().unwrap();
    assert_eq!(table.len(), 2);
    assert_eq!(table["key"].as_integer(), Some(1));
    assert_eq!(table["other"].as_str(), Some("two"));
}
