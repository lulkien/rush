use abi_stable::std_types::{RString, RVec};
use colored::*;
use rush_plugin::*;
use std::{
    env,
    io::{self, Write, stdout},
    sync::OnceLock,
};

static COMMAND_INFO: OnceLock<CommandInfo> = OnceLock::new();
static HOSTNAME: OnceLock<Option<String>> = OnceLock::new();
static USERNAME: OnceLock<Option<String>> = OnceLock::new();

fn get_plugin_info() -> &'static CommandInfo {
    COMMAND_INFO.get_or_init(|| CommandInfo {
        name: env!("CARGO_PKG_NAME").into(),
        description: env!("CARGO_PKG_DESCRIPTION").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        help: "This plugin shouldn't be called in a normal way".into(),
    })
}

fn get_hostname() -> &'static Option<String> {
    HOSTNAME.get_or_init(|| gethostname::gethostname().into_string().ok())
}

fn get_username() -> &'static Option<String> {
    USERNAME.get_or_init(|| env::var("USER").ok())
}

#[derive(Debug, Clone, Default)]
struct PromptBuilder {
    components: Vec<String>,
}

#[allow(unused)]
impl PromptBuilder {
    fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Add a component if it's not None/empty
    fn add_component<F>(&mut self, getter: F) -> &mut Self
    where
        F: FnOnce() -> Option<String>,
    {
        if let Some(component) = getter() {
            self.components.push(component);
        }
        self
    }

    /// Add a component with an icon
    fn add_with_icon<F>(&mut self, icon: &str, getter: F) -> &mut Self
    where
        F: FnOnce() -> Option<String>,
    {
        if let Some(value) = getter() {
            self.components.push(format!("{} {}", icon, value));
        }
        self
    }

    /// Add a static component (always added)
    fn add_static(&mut self, component: &str) -> &mut Self {
        self.components.push(component.to_string());
        self
    }

    /// Add a component with conditional formatting
    fn add_with_format<F>(
        &mut self,
        getter: F,
        formatter: impl FnOnce(String) -> String,
    ) -> &mut Self
    where
        F: FnOnce() -> Option<String>,
    {
        if let Some(value) = getter() {
            self.components.push(formatter(value));
        }
        self
    }

    /// Build the prompt string
    fn build(&self) -> String {
        if self.components.is_empty() {
            return String::new();
        }

        // Format: "<space><component1><space><component2><space>..."
        let mut result = String::new();
        for component in &self.components {
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(component);
        }

        // Add trailing space after all components
        if !result.is_empty() {
            result.push(' ');
        }

        result
    }

    /// Build and print the prompt
    fn display(&self) -> io::Result<()> {
        write!(stdout(), "{}", self.build())
    }
}

#[info]
pub fn info() -> CommandInfo {
    get_plugin_info().clone()
}

#[desc]
pub fn desc() -> RString {
    get_plugin_info().description.clone()
}

#[help]
pub fn help() -> RString {
    get_plugin_info().help.clone()
}

#[version]
pub fn version() -> RString {
    get_plugin_info().version.clone()
}

#[exec]
pub fn exec(_args: RVec<RString>) -> ExecResult {
    let username = get_username().clone();
    let hostname = get_hostname().clone();
    let home_path = dirs::home_dir();

    let is_root = username.as_deref() == Some("root");

    let mut prompt = PromptBuilder::new();

    // Add user component with icon and color
    prompt.add_with_format(
        || username.clone(),
        |user| {
            let user_icon = "";
            if is_root {
                format!("{} {}", user_icon, user).red().to_string()
            } else {
                format!("{} {}", user_icon, user).cyan().to_string()
            }
        },
    );

    // Add hostname component with icon
    prompt.add_with_format(
        || hostname.clone(),
        |host| {
            let host_icon = "󰟀"; // Alternative: "󰒋", "󰅐"
            format!("{} {}", host_icon, host).yellow().to_string()
        },
    );

    // Add directory component with icon and ~ substitution
    let current_dir = env::current_dir().unwrap_or_default();
    let dir_string = home_path
        .as_ref()
        .and_then(|home| current_dir.strip_prefix(home).ok())
        .map_or_else(
            || current_dir.to_string_lossy().to_string(),
            |relative| {
                if relative.as_os_str().is_empty() {
                    "~".to_string()
                } else {
                    format!("~/{}", relative.to_string_lossy())
                }
            },
        );

    prompt.add_with_format(
        || Some(dir_string.clone()),
        |dir| {
            let dir_icon = ""; // Alternative: "󰉋", "󰋜"
            format!("{} {}", dir_icon, dir).blue().to_string()
        },
    );

    // Add command indicator
    let indicator = if is_root { "#" } else { "$" };
    prompt.add_with_format(|| Some(indicator.to_string()), |ind| ind.to_string());

    ExecResult::new(0, &prompt.build())
}

#[load]
pub fn load() {
    get_plugin_info();
    get_hostname();
    get_username();
}
