use anyhow::Result;
use clap::Parser;
use srsglass::{
    download_dump, get_governorless_regions, get_passwordless_regions, parse_dump, save_to_excel,
};
use std::path::Path;

/// A command-line utility for generating NationStates region update timesheets
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// The name of your nation, to identify you to NationStates
    #[arg(short = 'n', long = "nation")]
    user_nation: String,

    /// Name of the output file
    #[arg(short, long, default_value = "srsglass.xlsx")]
    outfile: String,

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
        download_dump(&agent, dump_path)?;
    }

    println!("Parsing data dump");

    let regions = parse_dump(dump_path)?;

    let total_population = regions
        .last()
        .and_then(|region| {
            region
                .population
                .zip(region.nations_before)
                .map(|(population, nations_before)| population + nations_before)
        })
        .expect("Could not find total world population");

    println!("Saving timesheet");

    save_to_excel(
        regions.into_iter(),
        total_population,
        &args.outfile,
        args.major_length,
        args.minor_length,
        get_governorless_regions(&agent)?,
        get_passwordless_regions(&agent)?,
    )?;

    println!("Saved timesheet to {}", args.outfile);

    // let governorless =
    // let passwordless = get_passwordless_regions(&agent)?;

    // dbg!(governorless);
    // dbg!(passwordless);

    Ok(())
}
