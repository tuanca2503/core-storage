use crate::{
    cli::{
        command::{Command, FlagDef},
        context::CommandContext,
        result::CommandResult,
    },
    commands::*,
};

pub const GLOBAL_FLAGS: [&str; 4] = ["pretty", "verbose", "color", "sort"];

pub fn is_global(name: &str) -> bool {
    GLOBAL_FLAGS.contains(&name)
}

pub const REGISTRY_COMMANDS: [Command; 2] = [
    Command {
        name: "list",
        min_arguments: 0,
        max_arguments: 0,
        flags: &[FlagDef {
            name: "disk",
            description: "Show details",
        }],
        handler: list::list,
    },
    Command {
        name: "format",
        min_arguments: 1,
        max_arguments: 2,
        flags: &[FlagDef {
            name: "forced",
            description: "Ignore warning",
        }],
        handler: format::format,
    },
];

pub const HELP_TEXT: &str = "\
corestorage - V1

USAGE:
    corestorage <command> [args] [--flags]

COMMANDS:
    list                 Show storage
        --disk           Show detail disk

    format <disk name> <mode=quick,zero> Format disk 
        --forced         Ignore warning force format

GLOBAL FLAGS (ap dung moi command):
    --pretty             In ket qua dang bang de doc
    --verbose            In them log chi tiet
    --color              Enable output color
    --sort               Sort result
";

pub fn execute(ctx: CommandContext) -> CommandResult {
    let command = match REGISTRY_COMMANDS.iter().find(|c| c.name == ctx.command) {
        Some(c) => c,
        None => return CommandResult::Error(format!("Unknown command '{}'", ctx.command)),
    };

    if ctx.prm_count() < command.min_arguments || ctx.prm_count() > command.max_arguments {
        return CommandResult::Error(usage(command));
    }
    (command.handler)(ctx)
}

pub fn usage(command: &Command) -> String {
    let args = if command.min_arguments == 0 && command.max_arguments == 0 {
        String::new()
    } else {
        " <arg>".to_string()
    };
    format!("Usage: 'corestorage {}{}'", command.name, args)
}
