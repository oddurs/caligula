//! Fitting text into a fixed column.
//!
//! Two shapes, because the meaning sits at different ends. A branch name is
//! distinguished by its tail — `feat/0041-attach` and `feat/0049-classify`
//! differ nowhere else — so it keeps the tail. Prose is read from the front, so
//! it keeps the head.

/// Middle ellipsis, keeping the end: for identifiers.
pub fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "…".into();
    }
    let keep = max - 1;
    let head = keep / 3;
    let tail = keep - head;
    let chars: Vec<char> = s.chars().collect();
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[count - tail..]);
    out
}

/// Trailing ellipsis, keeping the beginning: for prose.
pub fn clip(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "…".into();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::{clip, truncate};

    #[test]
    fn truncate_keeps_the_tail() {
        assert_eq!(truncate("feat/0041-attach", 30), "feat/0041-attach");
        assert_eq!(truncate("feat/0041-attach", 10).chars().count(), 10);
        assert!(truncate("feat/0041-attach", 10).ends_with("attach"));
        assert_eq!(truncate("abc", 0), "");
    }

    #[test]
    fn clip_keeps_the_head() {
        assert!(clip("fix: a long commit subject", 12).starts_with("fix: a long"));
        assert_eq!(clip("fix: a long commit subject", 12).chars().count(), 12);
        assert_eq!(clip("short", 12), "short");
        // No room is no text, not an ellipsis: a column of zero width that
        // renders one character is a column that widens its row.
        assert_eq!(clip("anything", 0), "");
    }
}
