//! Shell variable store — all values are `Vec<String>` internally.
//!
//! - `VAR=value` stores `["value"]`
//! - `PATH=/a:/b` stores `["/a", "/b"]`
//! - Expansion joins parts with `:` to produce the expanded string.
//! - `${?}` reads from the special `?` key (updated via `set_exit_code`).

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
    // Only `${VAR}`, `${?}`, `$$` are supported.

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
                        let mut name = String::new();
                        while let Some(&nc) = chars.peek() {
                            if nc == '}' {
                                chars.next();
                                break;
                            }
                            name.push(nc);
                            chars.next();
                        }
                        result.push_str(&self.expand(&name));
                    }
                    _ => {
                        // Bare $ not followed by $, ?, or { — literal.
                        result.push('$');
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
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
    fn brace_expansion() {
        let v = vs();
        v.set("X", vec!["hello".to_string()]);
        assert_eq!(v.expand_string("${X}"), "hello");
    }

    #[test]
    fn brace_unset_is_empty() {
        let v = vs();
        assert_eq!(v.expand_string("${NOPE}"), "");
    }

    #[test]
    fn dollar_dollar_is_pid() {
        let v = vs();
        let result = v.expand_string("$$");
        let pid = std::process::id().to_string();
        assert_eq!(result, pid);
    }

    #[test]
    fn dollar_question_is_exit_code() {
        let v = vs();
        v.set_exit_code(42);
        assert_eq!(v.expand_string("${?}"), "42");
    }

    #[test]
    fn bare_dollar_is_literal() {
        let v = vs();
        assert_eq!(v.expand_string("$"), "$");
        assert_eq!(v.expand_string("$foo"), "$foo");
    }

    #[test]
    fn literal_dollar_brace_is_left_alone() {
        // ${X:-default} — modifier syntax not supported; everything after : is literal.
        let v = vs();
        assert_eq!(v.expand_string("${X:-default}"), "");
    }

    #[test]
    fn var_in_text() {
        let v = vs();
        v.set("X", vec!["world".to_string()]);
        assert_eq!(v.expand_string("hello ${X}!"), "hello world!");
    }
}
