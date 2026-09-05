//! Grapheme-aware presentation clipping. Limits count user-perceived characters;
//! ASCII and CJK retain their previous budgets, while a joined emoji stays atomic.
use unicode_segmentation::UnicodeSegmentation;

/// Iterate extended grapheme clusters for presentation measurement and wrapping.
pub fn graphemes(value: &str) -> impl DoubleEndedIterator<Item = &str> + Clone {
    value.graphemes(true)
}

/// Take up to `limit` extended grapheme clusters without allocating or splitting a cluster.
pub fn truncate_graphemes(value: &str, limit: usize) -> &str {
    let end = value
        .grapheme_indices(true)
        .nth(limit)
        .map_or(value.len(), |(offset, _)| offset);
    &value[..end]
}

/// Preserve a prefix, including the ellipsis in the total grapheme budget.
pub fn ellipsize_end(value: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }
    if value.graphemes(true).nth(limit).is_none() {
        return value.to_string();
    }
    let prefix = truncate_graphemes(value, limit - 1);
    format!("{prefix}…")
}

/// Preserve the existing 58% head / remaining tail split, counting the ellipsis in the budget.
pub fn ellipsize_middle(value: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }
    if value.graphemes(true).nth(limit).is_none() {
        return value.to_string();
    }
    let head_length = (((limit - 1) as f64) * 0.58).ceil() as usize;
    let tail_length = limit.saturating_sub(1 + head_length);
    let head = truncate_graphemes(value, head_length);
    let tail = if tail_length == 0 {
        ""
    } else {
        let start = value
            .grapheme_indices(true)
            .rev()
            .nth(tail_length - 1)
            .map_or(0, |(offset, _)| offset);
        &value[start..]
    };
    format!("{head}…{tail}")
}

/// Remove standalone preview markup, preserving symbols that form an emoji or accented cluster.
pub fn strip_preview_markup(value: &str) -> String {
    value
        .graphemes(true)
        .filter(|cluster| !matches!(*cluster, "\x60" | "*" | "_" | "~" | "|"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_grapheme_boundaries_preserve_cjk_latin_joiners_modifiers_flags_and_accents() {
        for cluster in ["中", "A", "👩‍💻", "👍🏽", "🇨🇳", "e\u{301}"] {
            let value = format!("{cluster}bcde{cluster}");
            assert_eq!(truncate_graphemes(&value, 1), cluster);
            assert_eq!(ellipsize_end(&value, 3), format!("{cluster}b…"));
            assert_eq!(ellipsize_middle(&value, 4), format!("{cluster}b…{cluster}"));
        }
    }

    #[test]
    fn unicode_grapheme_clipping_keeps_exact_limits_and_the_existing_ellipsis_styles() {
        assert_eq!(ellipsize_end("甲乙丙丁戊己庚", 5), "甲乙丙丁…");
        assert_eq!(ellipsize_middle("甲乙丙丁戊己庚", 5), "甲乙丙…庚");
        assert_eq!(ellipsize_middle("abcdefg", 5), "abc…g");
        assert_eq!(truncate_graphemes("👨‍👩‍👧‍👦x", 1), "👨‍👩‍👧‍👦");
        assert_eq!(ellipsize_end("👨‍👩‍👧‍👦x", 2), "👨‍👩‍👧‍👦x");
        assert_eq!(ellipsize_middle("👨‍👩‍👧‍👦x", 2), "👨‍👩‍👧‍👦x");
        assert_eq!(ellipsize_end("👨‍👩‍👧‍👦x", 1), "…");
        assert_eq!(ellipsize_middle("👨‍👩‍👧‍👦x", 1), "…");
        assert_eq!(truncate_graphemes("中文", 0), "");
        assert_eq!(ellipsize_end("中文", 0), "");
        assert_eq!(ellipsize_middle("中文", 0), "");
    }

    #[test]
    fn unicode_grapheme_output_never_exceeds_the_display_budget() {
        let value = "中文ABC👨‍👩‍👧‍👦👍🏽🇨🇳e\u{301}尾";
        for limit in 0..=20 {
            for result in [
                truncate_graphemes(value, limit).to_string(),
                ellipsize_end(value, limit),
                ellipsize_middle(value, limit),
            ] {
                assert!(result.graphemes(true).count() <= limit);
                for cluster in result.graphemes(true).filter(|cluster| *cluster != "…") {
                    assert!(value.graphemes(true).any(|original| original == cluster));
                }
            }
        }
    }

    #[test]
    fn preview_markup_cleanup_keeps_keycap_and_accented_clusters_intact() {
        assert_eq!(
            strip_preview_markup("*️⃣ **中文** _code_ \x60run\x60"),
            "*️⃣ 中文 code run"
        );
        assert_eq!(strip_preview_markup("_\u{301}~\u{301}"), "_\u{301}~\u{301}");
    }
}
