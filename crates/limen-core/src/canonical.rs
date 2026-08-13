//! Single implementation of canonical JSON encoding and SHA-256 digesting, used by every phase
//! of Limen (scenario `content_digest`, trial `result_digest`, schema-freshness checks, and any
//! future digesting need) so that all digests are computed identically everywhere.
//!
//! # Canonicalization
//!
//! [`sort_json_value`] converts a `serde_json::Value` into an equivalent value whose object keys
//! are recursively sorted lexicographically. We do this **explicitly**, walking the tree and
//! rebuilding every object through a `BTreeMap<String, Value>`, even though `serde_json::Map`
//! already behaves like a `BTreeMap` under its default feature set. We do this because if the
//! `preserve_order` feature of `serde_json` is ever turned on by *any* crate anywhere in the
//! final dependency graph (Cargo unifies features across the whole build), `serde_json::Map`
//! silently switches its backing store to an insertion-ordered `IndexMap`, which would silently
//! break byte-for-byte digest stability without a compile error. Explicitly funneling every
//! object through our own `BTreeMap` during canonicalization guards against that regardless of
//! which `serde_json::Map` implementation is active.
//!
//! Canonical bytes are produced with [`serde_json::to_vec`], which is `serde_json`'s compact
//! formatter (no pretty-printing whitespace).
//!
//! # Digesting
//!
//! [`digest_bytes`] hashes with SHA-256 (the `sha2` crate) and formats the result as
//! lowercase hex prefixed with `sha256:`, e.g. `sha256:2c26b46b...`.
//!
//! # Digesting a value that embeds its own digest
//!
//! Both `ScenarioManifest::content_digest` and `TrialResult::result_digest` are digests *of the
//! structure they are a field of*. To digest such a value, the digest field itself must be
//! entirely absent from the tree while hashing (not present-but-blank, since an empty string is
//! still a value and would itself affect the hash) and then inserted afterward.
//! [`digest_value_with_field_blanked`] and its generic convenience wrapper
//! [`digest_with_field_blanked`] do exactly this: they serialize the value, remove the named
//! top-level key from the resulting JSON object, canonicalize, and hash the remainder. Every
//! phase must compute `content_digest`/`result_digest` through one of these two functions so the
//! convention never drifts.
//!
//! # No silent Unicode normalization
//!
//! Nothing in this module (or anywhere else in `limen-core`) applies Unicode normalization (e.g.
//! NFC) to scenario text. Scenario text is hashed, tokenized, and byte-sliced exactly as authored.
//! This is a deliberate decision, not an oversight: silently normalizing text before hashing would
//! make it possible for two visibly-different-but-canonically-equal strings to collide, and would
//! make byte offsets recorded by scenario authors (in their original editor/tool) potentially
//! mismatch what gets hashed. If a future phase ever needs normalization, it must be an explicit,
//! documented step applied *before* text reaches this module, never inside it.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::EngineError;

/// Recursively sorts the keys of every JSON object in `value`, rebuilding each one through a
/// `BTreeMap<String, Value>` (see module docs for why this is done explicitly). Array order and
/// scalar values are left untouched. Pure data transform; cannot fail.
pub fn sort_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, sort_json_value(v)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sort_json_value).collect()),
        other => other,
    }
}

/// Canonicalizes an already-constructed `serde_json::Value` (sorted keys, compact encoding) and
/// returns the resulting UTF-8 bytes.
pub fn canonical_bytes_from_value(value: Value) -> Result<Vec<u8>, EngineError> {
    let sorted = sort_json_value(value);
    serde_json::to_vec(&sorted).map_err(|e| EngineError::Canonicalization(e.to_string()))
}

/// Converts any `Serialize` value to a `serde_json::Value` and canonicalizes it, returning the
/// resulting UTF-8 bytes.
pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, EngineError> {
    let raw =
        serde_json::to_value(value).map_err(|e| EngineError::Canonicalization(e.to_string()))?;
    canonical_bytes_from_value(raw)
}

/// Canonicalizes `value` (sorted keys, compact encoding) and returns it as a `String` rather than
/// raw bytes -- the form the WASM boundary hands back to the browser as "the exact canonical JSON
/// string" for an artifact (e.g. a [`crate::result::TrialResult`]) that a caller may download
/// byte-for-byte. `serde_json`'s compact writer always emits valid UTF-8 for values built only
/// from Rust's UTF-8-guaranteed `String`/`str` data (true of every type in this crate), so the
/// `String::from_utf8` conversion below cannot actually fail in practice, but is kept as a
/// `Result` rather than an `.expect()` per the crate-wide "no panics escaping a public function"
/// rule.
pub fn canonical_json_string<T: Serialize>(value: &T) -> Result<String, EngineError> {
    let bytes = canonical_bytes(value)?;
    String::from_utf8(bytes).map_err(|e| EngineError::Canonicalization(e.to_string()))
}

/// SHA-256-hashes `bytes` and formats the result as `sha256:<64 lowercase hex chars>`.
pub fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    format!("sha256:{}", to_hex_lower(&hash))
}

fn to_hex_lower(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX_DIGITS[(byte >> 4) as usize] as char);
        out.push(HEX_DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Canonicalizes and digests `value` as-is (no field blanking). Useful for digesting values that
/// do not embed their own digest (e.g. schema freshness checks).
pub fn digest_value<T: Serialize>(value: &T) -> Result<String, EngineError> {
    Ok(digest_bytes(&canonical_bytes(value)?))
}

/// Removes the top-level key `field` from `value` (if `value` is a JSON object and has that key),
/// then canonicalizes and digests the remainder. This is the low-level primitive described in the
/// module docs: the field is fully absent from the tree while hashing, not merely blanked.
pub fn digest_value_with_field_blanked(
    mut value: Value,
    field: &str,
) -> Result<String, EngineError> {
    if let Value::Object(map) = &mut value {
        map.remove(field);
    }
    Ok(digest_bytes(&canonical_bytes_from_value(value)?))
}

/// Generic convenience wrapper around [`digest_value_with_field_blanked`] for the common case of
/// digesting a typed struct (e.g. `&ScenarioManifest`) rather than a raw `serde_json::Value`.
///
/// This is the function every phase should call to compute `content_digest`/`result_digest`.
pub fn digest_with_field_blanked<T: Serialize>(
    value: &T,
    field: &str,
) -> Result<String, EngineError> {
    let raw =
        serde_json::to_value(value).map_err(|e| EngineError::Canonicalization(e.to_string()))?;
    digest_value_with_field_blanked(raw, field)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sort_json_value_orders_nested_object_keys() {
        let mut a = serde_json::Map::new();
        a.insert("z".to_string(), json!(1));
        a.insert("a".to_string(), json!({"y": 2, "b": 3}));
        let sorted = sort_json_value(Value::Object(a));
        let bytes = serde_json::to_vec(&sorted).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(text, r#"{"a":{"b":3,"y":2},"z":1}"#);
    }

    #[test]
    fn canonical_bytes_identical_regardless_of_field_construction_order() {
        // Two structurally-identical objects, built by inserting fields in different orders.
        let mut first = serde_json::Map::new();
        first.insert("alpha".to_string(), json!("one"));
        first.insert("beta".to_string(), json!(2));
        first.insert(
            "gamma".to_string(),
            json!({"nested_b": true, "nested_a": [3, 2, 1]}),
        );

        let mut second = serde_json::Map::new();
        second.insert(
            "gamma".to_string(),
            json!({"nested_a": [3, 2, 1], "nested_b": true}),
        );
        second.insert("beta".to_string(), json!(2));
        second.insert("alpha".to_string(), json!("one"));

        let first_bytes = canonical_bytes_from_value(Value::Object(first)).unwrap();
        let second_bytes = canonical_bytes_from_value(Value::Object(second)).unwrap();
        assert_eq!(first_bytes, second_bytes);
        assert_eq!(digest_bytes(&first_bytes), digest_bytes(&second_bytes));
    }

    #[test]
    fn mutating_any_single_field_flips_the_digest() {
        let base = json!({"a": 1, "b": [1, 2, 3], "c": {"d": "text"}});
        let base_digest = digest_value(&base).unwrap();

        let mutate_scalar = json!({"a": 2, "b": [1, 2, 3], "c": {"d": "text"}});
        let mutate_array = json!({"a": 1, "b": [1, 2, 4], "c": {"d": "text"}});
        let mutate_nested = json!({"a": 1, "b": [1, 2, 3], "c": {"d": "TEXT"}});

        assert_ne!(base_digest, digest_value(&mutate_scalar).unwrap());
        assert_ne!(base_digest, digest_value(&mutate_array).unwrap());
        assert_ne!(base_digest, digest_value(&mutate_nested).unwrap());
    }

    #[test]
    fn digest_format_is_sha256_prefixed_lowercase_hex() {
        let digest = digest_bytes(b"hello world");
        assert!(digest.starts_with("sha256:"));
        let hex = &digest["sha256:".len()..];
        assert_eq!(hex.len(), 64);
        assert!(hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        // Known SHA-256 of "hello world".
        assert_eq!(
            digest,
            "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn field_blanking_ignores_current_value_of_that_field() {
        let with_placeholder = json!({"content_digest": "sha256:0000", "a": 1, "b": 2});
        let with_different_placeholder =
            json!({"content_digest": "sha256:ffffffff", "a": 1, "b": 2});
        let without_field = json!({"a": 1, "b": 2});

        let d1 = digest_value_with_field_blanked(with_placeholder, "content_digest").unwrap();
        let d2 =
            digest_value_with_field_blanked(with_different_placeholder, "content_digest").unwrap();
        let d3 = digest_value(&without_field).unwrap();

        assert_eq!(d1, d2);
        assert_eq!(d1, d3);
    }

    #[test]
    fn canonical_json_string_matches_utf8_of_canonical_bytes() {
        let value = json!({"z": 1, "a": [3, 2, 1], "m": {"y": true, "x": "hello"}});
        let bytes = canonical_bytes_from_value(value.clone()).unwrap();
        let expected = String::from_utf8(bytes).unwrap();
        assert_eq!(canonical_json_string(&value).unwrap(), expected);
        assert_eq!(
            canonical_json_string(&value).unwrap(),
            r#"{"a":[3,2,1],"m":{"x":"hello","y":true},"z":1}"#
        );
    }

    #[test]
    fn generic_wrapper_matches_value_based_primitive() {
        #[derive(Serialize)]
        struct Sample {
            content_digest: String,
            title: String,
            count: u32,
        }

        let sample = Sample {
            content_digest: "sha256:stale".to_string(),
            title: "hello".to_string(),
            count: 7,
        };

        let via_generic = digest_with_field_blanked(&sample, "content_digest").unwrap();
        let via_value = digest_value_with_field_blanked(
            json!({"content_digest": "sha256:stale", "title": "hello", "count": 7}),
            "content_digest",
        )
        .unwrap();
        assert_eq!(via_generic, via_value);
    }
}
