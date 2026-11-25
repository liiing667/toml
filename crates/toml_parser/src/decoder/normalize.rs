//! Unicode normalization for keys
//!
//! TOML treats keys as opaque strings and compares them byte-for-byte.  That means two keys that
//! are visually identical but encoded with different [Unicode normalization forms] (for example
//! `é` as `U+00E9` versus `e` followed by `U+0301`) are treated as distinct keys.  Keys are
//! canonicalized to [NFC] while decoding so that canonically equivalent spellings share a single
//! key identity, matching how a reader perceives them.
//!
//! [Unicode normalization forms]: https://www.unicode.org/reports/tr15/
//! [NFC]: https://www.unicode.org/reports/tr15/#Norm_Forms

use alloc::borrow::Cow;
use alloc::string::String;

use unicode_normalization::UnicodeNormalization as _;
use unicode_normalization::is_nfc;

use crate::ErrorSink;
use crate::ParseError;
use crate::Span;
use crate::decoder::StringBuilder;

const ALLOCATION_ERROR: &str = "could not allocate for string";

/// Canonicalize a decoded key to NFC.
///
/// The fast path keeps the borrowed, zero-copy representation whenever the key is already in NFC
/// (which includes every ASCII-only key).  Only keys containing a non-canonical sequence are
/// re-encoded into an owned, normalized string.
///
/// TOML keys that contain no non-ASCII bytes cannot contain a combining sequence and are
/// necessarily already in NFC, so they skip the table lookups entirely.
pub(crate) fn normalize_key<'i>(key: &'i str) -> Cow<'i, str> {
    if key.is_ascii() || is_nfc(key) {
        Cow::Borrowed(key)
    } else {
        Cow::Owned(key.nfc().collect())
    }
}

/// A [`StringBuilder`] that canonicalizes decoded keys to NFC before forwarding them.
///
/// Keys made up entirely of ASCII bytes are forwarded directly so the common, zero-copy path is
/// untouched.  Any key containing non-ASCII bytes is buffered while it is decoded and normalized
/// once decoding completes, keeping the canonicalization logic in a single place regardless of
/// whether the key was quoted, escaped or unquoted.
pub(crate) struct NormalizingKeyBuilder<'b, 'i> {
    inner: &'b mut dyn StringBuilder<'i>,
    buffer: Option<String>,
}

impl<'b, 'i> NormalizingKeyBuilder<'b, 'i> {
    pub(crate) fn new(inner: &'b mut dyn StringBuilder<'i>, raw: &str) -> Self {
        // A key made up entirely of ASCII bytes cannot contain a combining sequence and is
        // necessarily already in NFC, so it can skip buffering and normalization.
        let buffer = (!raw.is_ascii()).then(String::new);
        Self { inner, buffer }
    }

    /// Flush the buffered, normalized key once decoding has finished.
    pub(crate) fn finish(self, error: &mut dyn ErrorSink, key_span: Span) {
        let Some(buffer) = self.buffer else {
            return;
        };
        let normalized = normalize_key(&buffer).into_owned();
        if !self.inner.push_owned(normalized) {
            error.report_error(ParseError::new(ALLOCATION_ERROR).with_unexpected(key_span));
        }
    }
}

impl<'i> StringBuilder<'i> for NormalizingKeyBuilder<'_, 'i> {
    fn clear(&mut self) {
        if let Some(buffer) = self.buffer.as_mut() {
            buffer.clear();
        } else {
            self.inner.clear();
        }
    }

    fn push_str(&mut self, append: &'i str) -> bool {
        if let Some(buffer) = self.buffer.as_mut() {
            buffer.push_str(append);
            true
        } else {
            self.inner.push_str(append)
        }
    }

    fn push_char(&mut self, append: char) -> bool {
        if let Some(buffer) = self.buffer.as_mut() {
            buffer.push(append);
            true
        } else {
            self.inner.push_char(append)
        }
    }

    fn push_owned(&mut self, append: String) -> bool {
        if let Some(buffer) = self.buffer.as_mut() {
            buffer.push_str(&append);
            true
        } else {
            self.inner.push_owned(append)
        }
    }
}

#[cfg(test)]
#[cfg(feature = "std")]
mod test {
    use alloc::borrow::Cow;

    use super::*;

    #[test]
    fn ascii_keys_are_not_canonicalized() {
        let key = "plain-ascii_123";
        assert!(matches!(normalize_key(key), Cow::Borrowed(k) if k == key));
    }

    #[test]
    fn nfc_keys_keep_their_borrowed_form() {
        let key = "caf\u{e9}";
        assert!(matches!(normalize_key(key), Cow::Borrowed(k) if k == key));
    }

    #[test]
    fn nfd_keys_are_canonicalized_to_nfc() {
        let nfd = "cafe\u{301}";
        let nfc = "caf\u{e9}";
        assert_eq!(normalize_key(nfd), Cow::Borrowed(nfc));
    }

    #[test]
    fn combining_marks_are_reordered_and_composed_canonically() {
        // U+0061 U+0300 U+0315 is not canonically ordered; NFC composes `a` + grave accent and
        // reorders the remaining combining mark.
        let unordered = "a\u{300}\u{315}";
        let normalized = normalize_key(unordered);
        assert_eq!(normalized, Cow::Borrowed("\u{e0}\u{315}"));
        assert!(matches!(normalized, Cow::Owned(_)));
    }

    #[test]
    fn builder_passes_ascii_through() {
        let mut output = Cow::Borrowed("");
        let mut builder = NormalizingKeyBuilder::new(&mut output, "ascii");
        assert!(builder.buffer.is_none());
        assert!(builder.push_str("ascii"));
        builder.finish(&mut (), Span::new_unchecked(0, 5));
        assert!(matches!(output, Cow::Borrowed("ascii")));
    }

    #[test]
    fn builder_normalizes_buffered_nfd_key() {
        let mut output = Cow::Borrowed("");
        let mut builder = NormalizingKeyBuilder::new(&mut output, "cafe\u{301}");
        assert!(builder.buffer.is_some());
        assert!(builder.push_str("cafe\u{301}"));
        builder.finish(&mut (), Span::new_unchecked(0, 6));
        assert_eq!(output, Cow::Borrowed("caf\u{e9}"));
        assert!(matches!(output, Cow::Owned(_)));
    }
}
