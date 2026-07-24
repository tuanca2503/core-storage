mod cli;
mod commands;
mod registry;

use cli::parser;
use cli::result::CommandResult;
use registry::{HELP_TEXT, execute};

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    let ctx = match parser::parse(&argv) {
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

fn render(result: CommandResult, global: Vec<String>) {
    match result {
        CommandResult::Text(s) => println!("{s}"),
        CommandResult::List(mut items) => {
            if global.iter().any(|s| s == "sort") {
                items.sort();
            }
            for item in items {
                println!("{item}");
            }
        }
        CommandResult::Table(mut rows) => {
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
        CommandResult::Error(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
