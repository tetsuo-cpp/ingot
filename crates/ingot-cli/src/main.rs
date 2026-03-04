// thiserror 2.x `Display` impl triggers `unused_assignments` for named fields.
#![allow(unused_assignments)]

use std::path::PathBuf;
use std::process;

use clap::Parser;
use miette::{Diagnostic, NamedSource};
use thiserror::Error;

use ingot_types::AsmError;

/// Ingot — ARM64 assembler for macOS
#[derive(Parser)]
#[command(name = "ingot", version)]
struct Args {
    /// Input assembly source file
    input: PathBuf,

    /// Output object file (defaults to input with .o extension)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// Wrapper that attaches source text to assembly errors so miette can render labels.
#[derive(Debug, Error, Diagnostic)]
#[error("assembly failed with {} error(s)", .errors.len())]
struct AssemblyReport {
    #[source_code]
    src: NamedSource<String>,
    #[related]
    errors: Vec<AsmError>,
}

fn main() {
    let args = Args::parse();

    let output = args
        .output
        .unwrap_or_else(|| args.input.with_extension("o"));

    let source = match std::fs::read_to_string(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read `{}`: {e}", args.input.display());
            process::exit(1);
        }
    };

    match ingot_asm::assemble(&source) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(&output, bytes) {
                eprintln!("error: cannot write `{}`: {e}", output.display());
                process::exit(1);
            }
        }
        Err(asm_errors) => {
            let filename = args.input.display().to_string();
            let report = AssemblyReport {
                src: NamedSource::new(filename, source),
                errors: asm_errors.errors,
            };
            eprintln!("{:?}", miette::Report::new(report));
            process::exit(1);
        }
    }
}
