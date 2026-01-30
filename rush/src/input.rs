use std::io;

use anyhow::Context;

pub fn get_user_input() -> anyhow::Result<String> {
    let mut user_input = String::new();
    io::stdin()
        .read_line(&mut user_input)
        .context("reading user input")?;
    Ok(user_input.trim().to_owned())
}
