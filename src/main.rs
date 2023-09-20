use clap::Parser;

/// A command-line utility for generating NationStates region update timesheets
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// The name of your nation, to identify you to NationStates
    #[arg(short = 'n', long = "nation")]
    user_nation: String,
}

fn main() {
    let args = Cli::parse();
    println!("Running srsglass with user nation {}", args.user_nation);
}
