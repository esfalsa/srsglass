use std::path::Path;

use anyhow::Result;
use clap::Parser;

/// A command-line utility for generating NationStates region update timesheets
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// The name of your nation, to identify you to NationStates
    #[arg(short = 'n', long = "nation")]
    user_nation: String,

    /// Path to the data dump
    #[arg(short = 'p', long = "path", default_value = "regions.xml.gz")]
    dump_path: String,

    /// Use the current data dump instead of downloading
    #[arg(short = 'd', long = "dump", default_value_t = false)]
    use_dump: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    println!("Running srsglass with user nation {}", args.user_nation);

    let user_agent = format!(
        "{}/{} (by:Esfalsa, usedBy:{})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        args.user_nation
    );

    let agent = ureq::AgentBuilder::new().user_agent(&user_agent).build();

    if args.use_dump && Path::new(&args.dump_path).exists() {
        println!("Using existing data dump");
    } else {
        println!("Downloading data dump");
        srsglass::download_dump(&agent, "regions.xml.gz")?;
    }

    Ok(())
}
