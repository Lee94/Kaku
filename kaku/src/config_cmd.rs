use anyhow::Context;
use clap::Parser;

use crate::config_tui;

#[derive(Debug, Parser, Clone, Default)]
pub struct ConfigCommand {
    /// Ensure ~/.config/kaku/kaku.lua exists, but do not open it.
    #[arg(long, hide = true)]
    ensure_only: bool,
}

impl ConfigCommand {
    pub fn run(&self) -> anyhow::Result<()> {
        let config_path = config::ensure_user_config_exists()?;
        if self.ensure_only {
            println!("Ensured config: {}", config_path.display());
            return Ok(());
        }

        // Launch TUI
        config_tui::run().context("config tui")
    }
}
