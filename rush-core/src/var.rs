//! Shell variable store — all values are `Vec<String>` internally.
//!
//! - `VAR=value` stores `["value"]`
//! - `PATH=/a:/b` stores `["/a", "/b"]`
//! - Expansion joins parts with `:` to produce the expanded string.
//! - `$?` reads from the special `?` key (updated via `set_exit_code`).

use dashmap::DashMap;

#[derive(Default)]
pub struct VarStore(DashMap<String, Vec<String>>);

impl VarStore {
    /// Get the full value list for a variable.
    pub fn get(&self, name: &str) -> Vec<String> {
        self.0
            .get(name)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// Set a variable (overwrites if it exists).
    pub fn set(&self, name: &str, value: Vec<String>) {
        self.0.insert(name.to_string(), value);
    }

    /// Set a variable from a colon-separated string (e.g. `PATH=/a:/b`).
    pub fn set_colon(&self, name: &str, value: &str) {
        let parts: Vec<String> = value.split(':').map(String::from).collect();
        self.set(name, parts);
    }

    /// Unset a variable.
    pub fn unset(&self, name: &str) -> Option<(String, Vec<String>)> {
        self.0.remove(name)
    }

    /// Update the `$?` exit code.
    pub fn set_exit_code(&self, code: i32) {
        self.set("?", vec![code.to_string()]);
    }

    /// Expand a variable: join all parts with `:`.
    /// Returns empty string if the variable is not set.
    pub fn expand(&self, name: &str) -> String {
        let parts = self.get(name);
        if parts.is_empty() {
            String::new()
        } else {
            parts.join(":")
        }
    }

    /// Expand `$VAR` references in a string.
    /// Supports: `$VAR`, `${VAR}`, `$?` (exit status), `$$` (pid).
    /// `$?` reads from the `?` variable (set via `set_exit_code`).
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
                        // Lonely $ at end of string or before non-name char — literal.
                        result.push('$');
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Build an environment array for execve (KEY=VALUE format).
    pub fn build_env_array(&self) -> Vec<std::ffi::CString> {
        self.0
            .iter()
            .filter(|entry| entry.key() != "?") // skip $? (internal)
            .map(|entry| {
                let pair = format!("{}={}", entry.key(), entry.value().join(":"));
                std::ffi::CString::new(pair).unwrap_or_default()
            })
            .collect()
    }
    #[allow(unused)]
    pub fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    /// Return all variable names (for export / debugging).
    #[allow(unused)]
    pub fn names(&self) -> Vec<String> {
        self.0.iter().map(|e| e.key().clone()).collect()
    }
}
