use super::context::CommandContext;
use super::result::CommandResult;

pub struct FlagDef {
    pub name: &'static str,
    pub description: &'static str,
}
pub struct Command {
    pub name: &'static str,
    pub min_arguments: usize,
    pub max_arguments: usize,
    pub flags: &'static [FlagDef],
    pub handler: fn(CommandContext) -> CommandResult,
}
