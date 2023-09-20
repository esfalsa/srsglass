use std::path::Path;

use anyhow::Result;
use clap::Parser;
use srsglass::parse_dump;

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

    let mut dump_path = Path::new(&args.dump_path);

    if args.use_dump && dump_path.exists() {
        println!("Using existing data dump");
    } else {
        println!("Downloading data dump");
        dump_path = Path::new("regions.xml.gz");
        srsglass::download_dump(&agent, dump_path)?;
    }

    println!("Parsing data dump");

    let regions = parse_dump(dump_path)?;

    Ok(())
}
