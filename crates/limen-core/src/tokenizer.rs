//! Bundled tokenizer `limen-lex@1.0.0`.
//!
//! Rules:
//! - A maximal contiguous run of ASCII `[A-Za-z0-9]` bytes is one token.
//! - Every other individual non-whitespace ASCII byte (punctuation/symbols, and any stray ASCII
//!   control byte) is its own one-byte token.
//! - Every individual non-ASCII Unicode scalar value (i.e. every `char` outside the ASCII range)
//!   is its own one token, regardless of how many UTF-8 bytes it occupies. A multi-scalar
//!   grapheme cluster (e.g. an emoji built from several scalars joined by ZWJ) therefore produces
//!   multiple tokens, one per scalar -- this tokenizer has no concept of grapheme clusters.
//! - ASCII whitespace -- space (0x20), tab (0x09), CR (0x0D), LF (0x0A) -- separates tokens and is
//!   never itself a token. No other ASCII byte is treated as whitespace.
//!
//! Spans are UTF-8 byte offsets `[byte_start, byte_end)` into the exact string passed to
//! [`tokenize`]/[`count_tokens`]. A span never splits a multi-byte codepoint: this falls out
//! naturally from iterating `char_indices()` and always ending a span at a `char` boundary.
//!
//! No Unicode normalization (e.g. NFC) is applied before tokenizing; text is tokenized exactly as
//! given (see the crate-level note in `canonical.rs` for why).
//!
//! # Budget accounting happens on emitted output, not raw source text
//!
//! This tokenizer must be used to count tokens of the *final assembled/emitted* context text a
//! strategy produces -- including any source-label prefixes or deterministic template wrapper
//! text the strategy adds -- **not** the raw, unmodified source item text. A strategy that adds a
//! wrapper such as `"[log_line #3] "` before an item's text must count tokens on the full
//! `"[log_line #3] " + text` string, not on `text` alone. Every later strategy-implementation and
//! metrics phase must respect this or budget accounting will silently disagree with what a real
//! model would actually be charged for.

/// One token's byte span, `[byte_start, byte_end)`, into the string that was tokenized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub byte_start: u32,
    pub byte_end: u32,
}

/// Stable identifier for this tokenizer's exact rule set. Bump the version suffix (and define a
/// new constant/tokenizer) if the rules ever change; never redefine what this exact string means.
pub const TOKENIZER_ID: &str = "limen-lex@1.0.0";

/// Walks `text` once, invoking `on_token(byte_start, byte_end)` for every token in order. Shared
/// by [`tokenize`] and [`count_tokens`] so the two can never disagree about what counts as a
/// token.
fn walk_tokens(text: &str, mut on_token: impl FnMut(u32, u32)) {
    let mut run_start: Option<u32> = None;

    for (byte_idx, ch) in text.char_indices() {
        let byte_idx = byte_idx as u32;
        if ch.is_ascii() {
            let byte = ch as u8;
            let is_alnum = byte.is_ascii_alphanumeric();
            let is_whitespace = matches!(byte, b' ' | b'\t' | b'\r' | b'\n');

            if is_alnum {
                if run_start.is_none() {
                    run_start = Some(byte_idx);
                }
                continue;
            }

            if let Some(start) = run_start.take() {
                on_token(start, byte_idx);
            }

            if !is_whitespace {
                on_token(byte_idx, byte_idx + 1);
            }
        } else {
            if let Some(start) = run_start.take() {
                on_token(start, byte_idx);
            }
            on_token(byte_idx, byte_idx + ch.len_utf8() as u32);
        }
    }

    if let Some(start) = run_start.take() {
        on_token(start, text.len() as u32);
    }
}

/// Tokenizes `text` per the `limen-lex@1.0.0` rules, returning every token's byte span in order.
pub fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    walk_tokens(text, |byte_start, byte_end| {
        tokens.push(Token {
            byte_start,
            byte_end,
        })
    });
    tokens
}

/// Counts tokens in `text` per the `limen-lex@1.0.0` rules without allocating a `Vec<Token>`.
/// Always equal to `tokenize(text).len() as u32`.
pub fn count_tokens(text: &str) -> u32 {
    let mut count: u32 = 0;
    walk_tokens(text, |_, _| count += 1);
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str) -> Vec<(u32, u32)> {
        tokenize(text)
            .into_iter()
            .map(|t| (t.byte_start, t.byte_end))
            .collect()
    }

    #[test]
    fn tokenizer_id_is_versioned() {
        assert_eq!(TOKENIZER_ID, "limen-lex@1.0.0");
    }

    #[test]
    fn empty_string_has_no_tokens() {
        assert_eq!(spans(""), vec![]);
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn pure_whitespace_has_no_tokens() {
        assert_eq!(spans(" \t\r\n \t "), vec![]);
        assert_eq!(count_tokens(" \t\r\n \t "), 0);
    }

    #[test]
    fn mixed_ascii_words_and_punctuation() {
        // "Hi, world!" -> "Hi" "," "world" "!"
        let text = "Hi, world!";
        assert_eq!(spans(text), vec![(0, 2), (2, 3), (4, 9), (9, 10)]);
        assert_eq!(count_tokens(text), 4);
        for (start, end) in spans(text) {
            // Every span must slice back to valid, non-empty UTF-8.
            assert!(!text[start as usize..end as usize].is_empty());
        }
    }

    #[test]
    fn digits_attached_to_letters_form_one_token() {
        let text = "abc123def 42";
        let toks = spans(text);
        assert_eq!(toks, vec![(0, 9), (10, 12)]);
        assert_eq!(&text[0..9], "abc123def");
        assert_eq!(&text[10..12], "42");
    }

    #[test]
    fn each_non_ascii_scalar_is_its_own_token_not_each_byte() {
        // "café" - 'é' is one scalar (2 UTF-8 bytes: 0xC3 0xA9), must be one token, not two.
        let text = "café";
        let toks = tokenize(text);
        assert_eq!(toks.len(), 2, "expected [\"caf\", \"é\"], got {toks:?}");
        assert_eq!(
            toks[0],
            Token {
                byte_start: 0,
                byte_end: 3
            }
        );
        assert_eq!(&text[0..3], "caf");
        assert_eq!(
            toks[1],
            Token {
                byte_start: 3,
                byte_end: 5
            }
        );
        assert_eq!(&text[3..5], "é");
    }

    #[test]
    fn multi_scalar_emoji_sequence_is_one_token_per_scalar() {
        // Family emoji U+1F468 U+200D U+1F469 U+200D U+1F467 (ZWJ-joined): 5 scalars, 5 tokens.
        let text = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let toks = tokenize(text);
        assert_eq!(toks.len(), 5);
        let expected_scalars = [
            '\u{1F468}',
            '\u{200D}',
            '\u{1F469}',
            '\u{200D}',
            '\u{1F467}',
        ];
        let mut cursor = 0usize;
        for (tok, expected_ch) in toks.iter().zip(expected_scalars.iter()) {
            assert_eq!(tok.byte_start, cursor as u32);
            let expected_len = expected_ch.len_utf8();
            assert_eq!(tok.byte_end, (cursor + expected_len) as u32);
            let slice = &text[tok.byte_start as usize..tok.byte_end as usize];
            assert_eq!(slice.chars().count(), 1);
            assert_eq!(slice.chars().next().unwrap(), *expected_ch);
            cursor += expected_len;
        }
        assert_eq!(cursor, text.len());
    }

    #[test]
    fn byte_spans_reconstruct_every_token_exactly_across_rich_mixed_text() {
        let text = "Report #42: temp=98.6°F, café ☕ — \"ok\"?\n(yes)";
        let toks = tokenize(text);
        // No span may be empty, spans must be strictly increasing/non-overlapping, and every
        // span must land on char boundaries (slicing must not panic).
        let mut prev_end = 0u32;
        for tok in &toks {
            assert!(tok.byte_start >= prev_end, "tokens overlap or go backwards");
            assert!(
                tok.byte_start < tok.byte_end,
                "token span must be non-empty"
            );
            assert!(text.is_char_boundary(tok.byte_start as usize));
            assert!(text.is_char_boundary(tok.byte_end as usize));
            let slice = &text[tok.byte_start as usize..tok.byte_end as usize];
            assert!(!slice.is_empty());
            prev_end = tok.byte_end;
        }
        assert_eq!(
            prev_end,
            text.len() as u32,
            "last token should reach end of non-whitespace text"
        );
    }

    #[test]
    fn token_count_invariant_matches_manual_group_count() {
        // "foo1, bar_2 baz3! café" is composed of exactly: 5 ASCII alnum runs ("foo1", "bar",
        // "2", "baz3", "caf"), 3 one-byte ASCII punctuation tokens (",", "_", "!"), and 1
        // non-ASCII scalar ("é") = 9 tokens total; whitespace contributes none. This is
        // intentionally simple enough to double as a property-test oracle later (e.g. generate
        // random (word|punct|scalar) sequences and assert count matches the number of generated
        // groups).
        let text = "foo1, bar_2 baz3! café";
        let expected_tokens = vec!["foo1", ",", "bar", "_", "2", "baz3", "!", "caf", "é"];
        let toks = tokenize(text);
        let actual: Vec<&str> = toks
            .iter()
            .map(|t| &text[t.byte_start as usize..t.byte_end as usize])
            .collect();
        assert_eq!(actual, expected_tokens);
        assert_eq!(count_tokens(text), expected_tokens.len() as u32);
        assert_eq!(count_tokens(text), tokenize(text).len() as u32);
    }

    #[test]
    fn count_tokens_always_agrees_with_tokenize_len() {
        let samples = [
            "",
            "   ",
            "hello, world!",
            "a1 b2 c3",
            "!!!???...",
            "mixed café ☕ text 123",
            "\u{1F468}\u{200D}\u{1F469}",
        ];
        for sample in samples {
            assert_eq!(
                count_tokens(sample),
                tokenize(sample).len() as u32,
                "mismatch for {sample:?}"
            );
        }
    }
}
