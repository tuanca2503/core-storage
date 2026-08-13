mod command;
mod commands;
mod registry;
use command::{Command, Context, Result};
use registry::{HELP_TEXT, execute, is_global};
use std::collections::HashMap;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    let ctx = match parse(&argv) {
        Some(ctx) => ctx,
        None => {
            eprintln!("Usage: corestorage <command> [args] [options]");
            std::process::exit(1);
        }
    };
    if ctx.command == "help" {
        print!("{HELP_TEXT}");
        return;
    }
    let global = ctx.global_options.clone();
    let result = execute(ctx);

    render(result, global);
}

fn render(result: Result, global: Vec<String>) {
    match result {
        Result::Text(s) => println!("{s}"),
        Result::Table(mut rows) => {
            if global.iter().any(|s| s == "sort") {
                rows.sort_by(|a, b| a.first().cmp(&b.first()));
            }
            let mut widths = vec![0; rows[0].len()];

            for row in &rows {
                for (i, cell) in row.iter().enumerate() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
            for row in rows {
                for (i, cell) in row.iter().enumerate() {
                    print!("{:<width$}  ", cell, width = widths[i]);
                }
                println!();
            }
        }
        Result::Error(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn parse(argv: &[String]) -> Option<Context> {
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
    Some(Context {
        command,
        params,
        options,
        global_options,
    })
}
