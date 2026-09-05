use super::super::{ellipsize_text_to_width, fit_text_to_lines, fit_text_to_width};
use crate::native_panel_core::resolve_estimated_text_width;
use echoisland_i18n::text::graphemes;

fn assert_complete_clusters(original: &str, rendered: &str) {
    for cluster in graphemes(rendered).filter(|cluster| !matches!(*cluster, "." | "\n")) {
        assert!(
            graphemes(original).any(|source| source == cluster),
            "split cluster {cluster:?} in {rendered:?}"
        );
    }
}

#[test]
fn text_clipping_and_wrapping_preserve_mixed_graphemes_at_each_width_boundary() {
    let text = "中文ABC123👩‍💻👍🏽🇨🇳e\u{301}👨‍👩‍👧‍👦尾";
    for width in (20..=240).map(f64::from) {
        for rendered in [
            fit_text_to_width(text, width, 10.0, 1),
            ellipsize_text_to_width(text, width, 10.0),
        ] {
            assert_complete_clusters(text, &rendered);
            assert!(resolve_estimated_text_width(&rendered, 10.0) <= width);
        }
        for max_lines in 1..=4 {
            let lines = fit_text_to_lines(text, width, 10.0, max_lines);
            assert!(lines.len() <= max_lines);
            for line in &lines {
                assert_complete_clusters(text, line);
                assert!(resolve_estimated_text_width(line, 10.0) <= width);
            }
        }
    }
}

#[test]
fn wrapping_moves_an_entire_emoji_to_the_next_line() {
    for cluster in ["👩‍💻", "👍🏽", "🇨🇳", "e\u{301}", "👨‍👩‍👧‍👦"]
    {
        let width = resolve_estimated_text_width(cluster, 10.0);
        assert_eq!(
            fit_text_to_lines(&format!("{cluster}{cluster}"), width, 10.0, 2),
            vec![cluster, cluster]
        );
    }
}

#[test]
fn omitted_body_content_always_has_an_ellipsis_even_when_the_line_has_spare_space() {
    assert_eq!(fit_text_to_lines("A👩‍💻", 30.0, 10.0, 1), vec!["A..."]);
    assert_eq!(fit_text_to_lines("👨‍👩‍👧‍👦", 30.0, 10.0, 2), vec!["..."]);
}

#[test]
fn wrapping_a_complete_body_does_not_mistake_trimmed_spaces_for_omitted_content() {
    assert_eq!(fit_text_to_lines("ab cd", 18.0, 10.0, 2), vec!["ab", "cd"]);
    assert_eq!(fit_text_to_width("  中文\n A  ", 120.0, 10.0, 1), "中文 A");
}
