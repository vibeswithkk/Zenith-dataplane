use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;

mod generators;
mod templates;

use generators::{plugin, ffi, schema};

#[derive(Parser)]
#[command(name = "zenith-codegen")]
#[command(about = "Code generator for Zenith Data Plane", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new WASM plugin template
    Plugin {
        /// Plugin name
        #[arg(short, long)]
        name: String,
        
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        
        /// Plugin type (filter, transform, aggregator)
        #[arg(short = 't', long = "type", default_value = "filter")]
        ptype: String,
    },
    
    /// Generate FFI bindings for a new language
    Ffi {
        /// Target language (go, python, node)
        #[arg(short, long)]
        lang: String,
        
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    
    /// Generate Arrow schema from JSON spec
    Schema {
        /// Schema definition file (JSON)
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output language (rust, python)
        #[arg(short, long, default_value = "rust")]
        lang: String,
        
        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Plugin { name, output, ptype } => {
            plugin::generate(&name, &output, &ptype)?;
            println!("[OK] Plugin '{}' generated at {:?}", name, output);
        }
        Commands::Ffi { lang, output } => {
            ffi::generate(&lang, &output)?;
            println!("[OK] FFI bindings for '{}' generated at {:?}", lang, output);
        }
        Commands::Schema { input, lang, output } => {
            schema::generate(&input, &lang, &output)?;
            println!("[OK] Schema code generated at {:?}", output);
        }
    }

    Ok(())
}
