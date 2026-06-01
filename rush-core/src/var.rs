//! Shell variable store — all values are `Vec<String>` internally.
//!
//! - `VAR=value` stores `["value"]`
//! - `PATH=/a:/b` stores `["/a", "/b"]`
//! - Expansion joins parts with `:` to produce the expanded string.
//! - `$?` reads from the special `?` key (updated via `set_exit_code`).

use std::cell::RefCell;
use std::collections::HashSet;

use dashmap::DashMap;

#[derive(Default)]
pub struct VarStore {
    vars: DashMap<String, Vec<String>>,
    /// Set of explicitly exported variable names.
    /// If empty, all variables are exported (default behavior).
    exported: RefCell<HashSet<String>>,
}

impl VarStore {
    // ── basic ops ────────────────────────────────────────────────

    pub fn get(&self, name: &str) -> Vec<String> {
        self.vars
            .get(name)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    pub fn set(&self, name: &str, value: Vec<String>) {
        self.vars.insert(name.to_string(), value);
    }

    pub fn set_colon(&self, name: &str, value: &str) {
        let parts: Vec<String> = value.split(':').map(String::from).collect();
        self.set(name, parts);
    }

    pub fn unset(&self, name: &str) -> Option<(String, Vec<String>)> {
        self.exported.borrow_mut().remove(name);
        self.vars.remove(name)
    }

    pub fn set_exit_code(&self, code: i32) {
        self.set("?", vec![code.to_string()]);
    }

    // ── export ───────────────────────────────────────────────────

    /// Mark a variable as exported.
    pub fn export(&self, name: &str) {
        self.exported.borrow_mut().insert(name.to_string());
    }

    /// Check if a variable is exported.
    /// If nothing has been explicitly exported, all vars are exported.
    pub fn is_exported(&self, name: &str) -> bool {
        let exported = self.exported.borrow();
        exported.is_empty() || exported.contains(name)
    }

    // ── expansion ────────────────────────────────────────────────

    pub fn expand(&self, name: &str) -> String {
        let parts = self.get(name);
        if parts.is_empty() {
            String::new()
        } else {
            parts.join(":")
        }
    }

    pub fn expand_string(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                match chars.peek() {
                    Some('$') => {
                        chars.next();
                        result.push_str(&std::process::id().to_string());
                    }
                    Some('?') => {
                        chars.next();
                        result.push_str(&self.expand("?"));
                    }
                    Some('{') => {
                        chars.next(); // consume '{'
                        result.push_str(&self.expand_braced(&mut chars));
                    }
                    Some(&nc) if nc.is_alphanumeric() || nc == '_' => {
                        let name = self.read_name(&mut chars);
                        result.push_str(&self.expand(&name));
                    }
                    _ => {
                        result.push('$');
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    // ── parameter expansion helpers ───────────────────────────────

    /// Read a variable name (alphanumeric + underscore) from the char stream.
    fn read_name(&self, chars: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> String {
        let mut name = String::new();
        while let Some(&nc) = chars.peek() {
            if nc.is_alphanumeric() || nc == '_' {
                name.push(nc);
                chars.next();
            } else {
                break;
            }
        }
        name
    }

    /// Read decimal digits from the char stream.
    fn read_digits(&self, chars: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> String {
        let mut digits = String::new();
        while let Some(&nc) = chars.peek() {
            if nc.is_ascii_digit() {
                digits.push(nc);
                chars.next();
            } else {
                break;
            }
        }
        digits
    }

    /// Read a braced word (until matching `}`), tracking brace depth.
    /// The outer `${` was already consumed; starts at depth 1.
    fn read_braced_word(
        &self,
        chars: &mut std::iter::Peekable<impl Iterator<Item = char>>,
    ) -> String {
        let mut depth: u32 = 1;
        let mut word = String::new();
        for c in chars.by_ref() {
            match c {
                '{' => {
                    depth += 1;
                    word.push(c);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    word.push(c);
                }
                _ => word.push(c),
            }
        }
        word
    }

    /// Expand a `${...}` expression (the `{` has already been consumed).
    /// Handles all POSIX parameter expansion modifiers.
    fn expand_braced(
        &self,
        chars: &mut std::iter::Peekable<impl Iterator<Item = char>>,
    ) -> String {
        // ${#VAR} — length
        if chars.peek() == Some(&'#') {
            chars.next();
            let name = self.read_name(chars);
            if chars.peek() == Some(&'}') {
                chars.next();
            }
            let value = self.expand(&name);
            return value.chars().count().to_string();
        }

        let name = self.read_name(chars);

        match chars.peek() {
            Some(&'}') => {
                chars.next();
                self.expand(&name)
            }
            Some(&':') => {
                chars.next(); // consume ':'
                match chars.peek() {
                    Some(&'-') => {
                        chars.next();
                        let word = self.read_braced_word(chars);
                        let value = self.expand(&name);
                        if value.is_empty() {
                            self.expand_string(&word)
                        } else {
                            value
                        }
                    }
                    Some(&'=') => {
                        chars.next();
                        let word = self.read_braced_word(chars);
                        let value = self.expand(&name);
                        if value.is_empty() {
                            let expanded = self.expand_string(&word);
                            self.set_colon(&name, &expanded);
                            expanded
                        } else {
                            value
                        }
                    }
                    Some(&'?') => {
                        chars.next();
                        let word = self.read_braced_word(chars);
                        let value = self.expand(&name);
                        if value.is_empty() {
                            let msg = if word.is_empty() {
                                format!("{}: parameter not set", name)
                            } else {
                                self.expand_string(&word)
                            };
                            eprintln!("rush: {msg}");
                            String::new()
                        } else {
                            value
                        }
                    }
                    Some(&'+') => {
                        chars.next();
                        let word = self.read_braced_word(chars);
                        let value = self.expand(&name);
                        if value.is_empty() {
                            String::new()
                        } else {
                            self.expand_string(&word)
                        }
                    }
                    Some(&c) if c.is_ascii_digit() => {
                        let offset: usize = self.read_digits(chars).parse().unwrap_or(0);
                        let length = if chars.peek() == Some(&':') {
                            chars.next();
                            self.read_digits(chars).parse::<usize>().ok()
                        } else {
                            None
                        };
                        if chars.peek() == Some(&'}') {
                            chars.next();
                        }

                        let value = self.expand(&name);
                        let vchars: Vec<char> = value.chars().collect();
                        let start = offset.min(vchars.len());
                        let end = match length {
                            Some(len) => (start + len).min(vchars.len()),
                            None => vchars.len(),
                        };
                        vchars[start..end].iter().collect()
                    }
                    _ => {
                        // Unknown modifier; consume rest until '}'.
                        let mut result = self.expand(&name);
                        result.push(':');
                        let mut depth: u32 = 1;
                        for c in chars.by_ref() {
                            match c {
                                '{' => depth += 1,
                                '}' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            result.push(c);
                        }
                        result
                    }
                }
            }
            Some(&'#') => {
                chars.next(); // consume '#'
                let double = chars.peek() == Some(&'#');
                if double {
                    chars.next();
                }

                let pattern = self.read_braced_word(chars);
                let value = self.expand(&name);
                if pattern.is_empty() {
                    return value;
                }
                let expanded = self.expand_string(&pattern);
                if double {
                    crate::glob::remove_longest_prefix(&value, &expanded)
                } else {
                    crate::glob::remove_shortest_prefix(&value, &expanded)
                }
            }
            Some(&'%') => {
                chars.next(); // consume '%'
                let double = chars.peek() == Some(&'%');
                if double {
                    chars.next();
                }

                let pattern = self.read_braced_word(chars);
                let value = self.expand(&name);
                if pattern.is_empty() {
                    return value;
                }
                let expanded = self.expand_string(&pattern);
                if double {
                    crate::glob::remove_longest_suffix(&value, &expanded)
                } else {
                    crate::glob::remove_shortest_suffix(&value, &expanded)
                }
            }
            _ => {
                // No closing brace — invalid, return empty.
                String::new()
            }
        }
    }

    // ── environment ──────────────────────────────────────────────

    pub fn build_env_array(&self) -> Vec<std::ffi::CString> {
        self.vars
            .iter()
            .filter(|entry| entry.key() != "?" && self.is_exported(entry.key()))
            .map(|entry| {
                let pair = format!("{}={}", entry.key(), entry.value().join(":"));
                std::ffi::CString::new(pair).unwrap_or_default()
            })
            .collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.vars.contains_key(name)
    }

    /// Return names of all exported variables (for `export` listing).
    pub fn exported_names(&self) -> Vec<String> {
        self.vars
            .iter()
            .filter(|e| e.key() != "?" && self.is_exported(e.key()))
            .map(|e| e.key().clone())
            .collect()
    }

    /// Get the first entry of a variable (for single-value vars like cache dir).
    pub fn first(&self, name: &str) -> String {
        self.get(name).first().cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vs() -> VarStore {
        VarStore::default()
    }

    #[test]
    fn basic_var_expansion() {
        let v = vs();
        v.set("X", vec!["hello".to_string()]);
        assert_eq!(v.expand_string("$X"), "hello");
        assert_eq!(v.expand_string("${X}"), "hello");
    }

    #[test]
    fn default_value_unset() {
        let v = vs();
        assert_eq!(v.expand_string("${X:-default}"), "default");
    }

    #[test]
    fn default_value_set() {
        let v = vs();
        v.set("X", vec!["value".to_string()]);
        assert_eq!(v.expand_string("${X:-default}"), "value");
    }

    #[test]
    fn assign_default_unset() {
        let v = vs();
        assert_eq!(v.expand_string("${X:=assigned}"), "assigned");
        assert_eq!(v.expand("X"), "assigned");
    }

    #[test]
    fn assign_default_set() {
        let v = vs();
        v.set("X", vec!["keep".to_string()]);
        assert_eq!(v.expand_string("${X:=assigned}"), "keep");
    }

    #[test]
    fn alternative_value_set() {
        let v = vs();
        v.set("X", vec!["value".to_string()]);
        assert_eq!(v.expand_string("${X:+alt}"), "alt");
    }

    #[test]
    fn alternative_value_unset() {
        let v = vs();
        assert_eq!(v.expand_string("${X:+alt}"), "");
    }

    #[test]
    fn error_unset() {
        let v = vs();
        // Prints to stderr, returns empty.
        assert_eq!(v.expand_string("${X:?not set}"), "");
    }

    #[test]
    fn string_length() {
        let v = vs();
        v.set("X", vec!["hello".to_string()]);
        assert_eq!(v.expand_string("${#X}"), "5");
    }

    #[test]
    fn substring() {
        let v = vs();
        v.set("X", vec!["hello".to_string()]);
        assert_eq!(v.expand_string("${X:1}"), "ello");
        assert_eq!(v.expand_string("${X:1:3}"), "ell");
    }

    #[test]
    fn prefix_removal_shortest() {
        let v = vs();
        v.set("X", vec!["abc/def/ghi.txt".to_string()]);
        assert_eq!(v.expand_string("${X#*/}"), "def/ghi.txt");
    }

    #[test]
    fn prefix_removal_longest() {
        let v = vs();
        v.set("X", vec!["abc/def/ghi.txt".to_string()]);
        assert_eq!(v.expand_string("${X##*/}"), "ghi.txt");
    }

    #[test]
    fn suffix_removal_shortest() {
        let v = vs();
        v.set("X", vec!["abc/def/ghi.txt".to_string()]);
        assert_eq!(v.expand_string("${X%/*}"), "abc/def");
    }

    #[test]
    fn suffix_removal_longest() {
        let v = vs();
        v.set("X", vec!["abc/def/ghi.txt".to_string()]);
        assert_eq!(v.expand_string("${X%%/*}"), "abc");
    }

    #[test]
    fn nested_braces_in_default() {
        let v = vs();
        v.set("Y", vec!["inner".to_string()]);
        // When X is unset, use ${Y} as default.
        assert_eq!(v.expand_string("${X:-${Y}}"), "inner");
    }
}
