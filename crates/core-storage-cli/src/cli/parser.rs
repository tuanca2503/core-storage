use std::collections::HashMap;

use crate::registry::is_global;

use super::context::CommandContext;

pub fn parse(argv: &[String]) -> Option<CommandContext> {
    let mut iter = argv.iter();
    let command = iter.next()?.clone();
    let mut params = Vec::new();
    let mut options = HashMap::new();
    let mut global_options = Vec::new();

    for token in iter {
        if let Some(rest) = token.strip_prefix("--") {
            if is_global(rest) {
                global_options.push(rest.to_string());
                continue;
            }

            match rest.split_once('=') {
                Some((name, value)) => {
                    options.insert(name.to_string(), Some(value.to_string()));
                }
                None => {
                    options.insert(rest.to_string(), None);
                }
            }
        } else {
            params.push(token.clone());
        }
    }
    Some(CommandContext {
        command,
        params,
        options,
        global_options,
    })
}
