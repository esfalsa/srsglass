use anyhow::Result;
use clap::Parser;
use srsglass::Client;
use std::path::Path;

/// A command-line utility for generating NationStates region update timesheets
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// The name of your nation, to identify you to NationStates
    #[arg(short = 'n', long = "nation")]
    user_nation: String,

    /// Name of the output file [default: srsglassYYYY-MM-DD.xlsx]
    #[arg(short, long)]
    outfile: Option<String>,

    /// Length of major update, in seconds
    #[arg(long = "major", default_value_t = 5350)]
    major_length: i32,

    /// Length of minor update, in seconds
    #[arg(long = "minor", default_value_t = 3550)]
    minor_length: i32,

    /// Use the current data dump instead of downloading
    #[arg(short = 'd', long = "dump", default_value_t = false)]
    use_dump: bool,

    /// Path to the data dump
    #[arg(short = 'p', long = "path", default_value = "regions.xml.gz")]
    dump_path: String,

    /// The number of milliseconds to use in timestamps
    #[arg(long = "precision", default_value_t = 0)]
    precision: i32,
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

    let client = Client::new(&user_agent);

    let dump_path = Path::new(&args.dump_path);

    let dump = if args.use_dump && dump_path.exists() {
        println!("Using existing data dump");
        client.get_dump_from_file(dump_path)?
    } else {
        println!("Downloading data dump");
        client.get_dump()?
    };

    println!("Saving timesheet");

    // Use dump's date to dynamically create the filename if none is specified
    let outfile = match args.outfile {
        Some(filepath) => filepath,
        None => format!("spyglass{}.xlsx", dump.dump_date),
    };

    dump.to_excel(
        &outfile,
        args.major_length,
        args.minor_length,
        args.precision,
    )?;

    println!("Saved timesheet to {}", outfile);

    Ok(())
}
