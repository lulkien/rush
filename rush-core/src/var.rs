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
                        chars.next();
                        let mut name = String::new();
                        while let Some(&nc) = chars.peek() {
                            if nc == '}' {
                                chars.next();
                                break;
                            }
                            if nc.is_alphanumeric() || nc == '_' {
                                name.push(nc);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        result.push_str(&self.expand(&name));
                    }
                    Some(&nc) if nc.is_alphanumeric() || nc == '_' => {
                        let mut name = String::new();
                        while let Some(&nc) = chars.peek() {
                            if nc.is_alphanumeric() || nc == '_' {
                                name.push(nc);
                                chars.next();
                            } else {
                                break;
                            }
                        }
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
