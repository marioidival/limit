use super::{Command, CommandContext, CommandResult};
use crate::error::CliError;
use crate::project_settings::ProjectSettings;

pub struct TldrCommand;

impl Command for TldrCommand {
    fn name(&self) -> &str {
        "tldr"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["warm"]
    }

    fn description(&self) -> &str {
        "Enable code analysis for this project (TLDR warm)"
    }

    fn usage(&self) -> Vec<&str> {
        vec!["/tldr", "/warm"]
    }

    fn execute(&self, _args: &str, ctx: &mut CommandContext) -> Result<CommandResult, CliError> {
        let project_path = ctx.project_path.clone();
        let settings = ProjectSettings::new()?;

        if settings.is_warm_enabled(&project_path) {
            ctx.add_system_message(
                "TLDR already enabled for this project. Indexes are being built in background."
                    .to_string(),
            );
            return Ok(CommandResult::Continue);
        }

        settings.set_warm_enabled(&project_path)?;

        ctx.add_system_message(
            "✓ TLDR enabled for this project. Building code indexes in background...\n\
             You can now use code analysis features (search, context, impact, etc.)."
                .to_string(),
        );

        Ok(CommandResult::TldrWarm)
    }
}
