//! Filename globbing (pathname expansion).
//!
//! Expands patterns containing `*`, `?`, and `[...]` into matching filenames.
//! Called after variable expansion on unquoted words.

/// Check if a word contains glob metacharacters.
pub fn has_glob_chars(word: &str) -> bool {
    word.contains('*') || word.contains('?') || word.contains('[')
}

/// Expand a glob pattern into matching paths.
/// Returns sorted, deduplicated matches. If no matches, returns the pattern itself.
pub fn glob_expand(pattern: &str) -> Vec<String> {
    if !has_glob_chars(pattern) {
        return vec![pattern.to_string()];
    }

    // Split by '/' to handle directory components.
    let parts: Vec<&str> = pattern.split('/').collect();
    let mut results = vec![String::new()];

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        let base_dirs = std::mem::take(&mut results);

        for base in &base_dirs {
            let search_dir = if base.is_empty() { "." } else { base.as_str() };

            if let Ok(entries) = std::fs::read_dir(search_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Skip hidden files unless the pattern explicitly starts with '.'.
                    if name.starts_with('.') && !part.starts_with('.') {
                        continue;
                    }

                    if glob_match(part, &name) {
                        let full = if base.is_empty() {
                            name
                        } else {
                            format!("{base}/{name}")
                        };
                        // Only include directories for non-final components.
                        if !is_last {
                            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                results.push(full);
                            }
                        } else {
                            results.push(full);
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() || (results.len() == 1 && results[0].is_empty()) {
        // No matches — return pattern literally (POSIX behavior).
        return vec![pattern.to_string()];
    }

    results.sort();
    results.dedup();
    results
}

/// Match a single path component against a glob pattern.
/// Supports `*` (any sequence), `?` (any single char), `[...]` (character class).
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let nam: Vec<char> = name.chars().collect();
    glob_match_slice(&pat, &nam, 0, 0)
}

fn glob_match_slice(pat: &[char], nam: &[char], pi: usize, ni: usize) -> bool {
    let mut pi = pi;
    let mut ni = ni;

    while pi < pat.len() {
        match pat[pi] {
            '*' => {
                // Try matching zero or more characters.
                if pi + 1 == pat.len() {
                    return true; // trailing * matches everything
                }
                for next_ni in ni..=nam.len() {
                    if glob_match_slice(pat, nam, pi + 1, next_ni) {
                        return true;
                    }
                }
                return false;
            }
            '?' => {
                if ni >= nam.len() {
                    return false;
                }
                pi += 1;
                ni += 1;
            }
            '[' => {
                if ni >= nam.len() {
                    return false;
                }
                let end = pat[pi..].iter().position(|&c| c == ']');
                let end = match end {
                    Some(e) => pi + e,
                    None => return false, // unmatched '[' — treat as literal
                };
                let class = &pat[pi + 1..end];
                let negate = class.first() == Some(&'!') || class.first() == Some(&'^');
                let class = if negate { &class[1..] } else { class };

                let mut matched = false;
                let mut ci = 0;
                while ci < class.len() {
                    if ci + 2 < class.len() && class[ci + 1] == '-' {
                        // Range: a-z
                        let start = class[ci] as u32;
                        let end = class[ci + 2] as u32;
                        let c = nam[ni] as u32;
                        if c >= start && c <= end {
                            matched = true;
                            break;
                        }
                        ci += 3;
                    } else {
                        if class[ci] == nam[ni] {
                            matched = true;
                            break;
                        }
                        ci += 1;
                    }
                }

                if matched == negate {
                    return false;
                }
                pi = end + 1;
                ni += 1;
            }
            _ => {
                if ni >= nam.len() || pat[pi] != nam[ni] {
                    return false;
                }
                pi += 1;
                ni += 1;
            }
        }
    }

    // Pattern exhausted — must have consumed all of name.
    ni == nam.len()
}

/// Remove the shortest prefix of `value` that matches `pattern` (glob).
/// Used for `${VAR#pattern}`.
pub fn remove_shortest_prefix(value: &str, pattern: &str) -> String {
    for i in 0..=value.len() {
        if glob_match(pattern, &value[..i]) {
            return value[i..].to_string();
        }
    }
    value.to_string()
}

/// Remove the longest prefix of `value` that matches `pattern` (glob).
/// Used for `${VAR##pattern}`.
pub fn remove_longest_prefix(value: &str, pattern: &str) -> String {
    let mut last_match = None;
    for i in 0..=value.len() {
        if glob_match(pattern, &value[..i]) {
            last_match = Some(i);
        }
    }
    match last_match {
        Some(i) => value[i..].to_string(),
        None => value.to_string(),
    }
}

/// Remove the shortest suffix of `value` that matches `pattern` (glob).
/// Used for `${VAR%pattern}`.
/// Note: does not consider the full value as a suffix (bash-compatible).
pub fn remove_shortest_suffix(value: &str, pattern: &str) -> String {
    // Check proper suffixes only: from shortest (rightmost) to longest.
    // Skip i = value.len() (empty suffix) and i = 0 (full value).
    for i in (1..value.len()).rev() {
        if glob_match(pattern, &value[i..]) {
            return value[..i].to_string();
        }
    }
    value.to_string()
}

/// Remove the longest suffix of `value` that matches `pattern` (glob).
/// Used for `${VAR%%pattern}`.
pub fn remove_longest_suffix(value: &str, pattern: &str) -> String {
    // Check suffixes from longest (full value) to shortest.
    // Includes full value (bash-compatible: %%*.txt on hello.txt → "").
    for i in 0..value.len() {
        if glob_match(pattern, &value[i..]) {
            return value[..i].to_string();
        }
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("*.rs", "foo.rs"));
        assert!(glob_match("*.rs", ".rs"));
        assert!(!glob_match("*.rs", "foo.txt"));
        assert!(glob_match("f*", "foo"));
        assert!(glob_match("*o", "foo"));
        assert!(glob_match("f*o", "foo"));
        assert!(glob_match("f*o", "f123o"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match("f?o", "foo"));
        assert!(glob_match("f?o", "fao"));
        assert!(!glob_match("f?o", "fo"));
        assert!(!glob_match("f?o", "fooo"));
    }

    #[test]
    fn test_glob_match_class() {
        assert!(glob_match("[abc]", "a"));
        assert!(glob_match("[abc]", "b"));
        assert!(!glob_match("[abc]", "d"));
        assert!(glob_match("[!abc]", "d"));
        assert!(!glob_match("[!abc]", "a"));
        assert!(glob_match("[a-z]", "m"));
        assert!(!glob_match("[a-z]", "9"));
    }

    #[test]
    fn test_has_glob_chars() {
        assert!(has_glob_chars("*.rs"));
        assert!(has_glob_chars("foo?"));
        assert!(has_glob_chars("[ab]"));
        assert!(!has_glob_chars("hello"));
    }

    #[test]
    fn test_remove_shortest_prefix() {
        assert_eq!(remove_shortest_prefix("abc/def/ghi.txt", "*/"), "def/ghi.txt");
        assert_eq!(remove_shortest_prefix("file.txt", "f*"), "ile.txt");
    }

    #[test]
    fn test_remove_longest_prefix() {
        assert_eq!(remove_longest_prefix("abc/def/ghi.txt", "*/"), "ghi.txt");
        // "f*" matches "file.txt" entirely — full value is a valid prefix match.
        assert_eq!(remove_longest_prefix("file.txt", "f*"), "");
    }

    #[test]
    fn test_remove_shortest_suffix() {
        assert_eq!(remove_shortest_suffix("abc/def/ghi.txt", "/*.txt"), "abc/def");
    }

    #[test]
    fn test_remove_longest_suffix() {
        assert_eq!(remove_longest_suffix("abc/def/ghi.txt", "/*.txt"), "abc");
    }
}
