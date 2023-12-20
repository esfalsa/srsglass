# srsglass

**srsglass** is a command-line utility for generating Excel workbooks with estimated update times of NationStates regions.

srsglass originated as a project to rewrite [Spyglass](https://github.com/derpseh/spyglass) in Rust. While it continues to share the same core functionality, some functionality has not been retained and some behavior has been modified.

## Installation

```sh
cargo install --git https://github.com/esfalsa/srsglass
```

## Usage

```
$ srsglass -h
A command-line utility for generating NationStates region update timesheets

Usage: srsglass [OPTIONS] --nation <USER_NATION>

Options:
  -n, --nation <USER_NATION>   The name of your nation, to identify you to NationStates
  -o, --outfile <OUTFILE>      Name of the output file [default: srsglass.xlsx]
      --major <MAJOR_LENGTH>   Length of major update, in seconds [default: 5350]
      --minor <MINOR_LENGTH>   Length of minor update, in seconds [default: 3550]
  -d, --dump                   Use the current data dump instead of downloading
  -p, --path <DUMP_PATH>       Path to the data dump [default: regions.xml.gz]
      --precision <PRECISION>  The number of milliseconds to use in timestamps [default: 0]
  -h, --help                   Print help
  -V, --version                Print version
```

## Performance

Here's a quick benchmark, run using [hyperfine](https://github.com/sharkdp/hyperfine).

```
Benchmark 1: ./Spyglass-v3.0.3-macOS.intel -n Esfalsa --dump
  Time (mean ± σ):     38.960 s ±  2.209 s    [User: 22.386 s, System: 1.180 s]
  Range (min … max):   35.656 s … 43.107 s    10 runs

Benchmark 2: srsglass -n Esfalsa --dump
  Time (mean ± σ):      5.086 s ±  1.093 s    [User: 2.278 s, System: 0.070 s]
  Range (min … max):    4.239 s …  8.055 s    10 runs

Summary
  srsglass -n Esfalsa --dump ran
    7.66 ± 1.70 times faster than ./Spyglass-v3.0.3-macOS.intel -n Esfalsa --dump
```

Note that this is just one benchmark on one machine. srsglass has not been extensively benchmarked, nor has it been extensively optimized for performance, so performance improvements compared to Spyglass are mainly attributable to differences at the language level between Rust and Python.

## License

[AGPL-3.0](./LICENSE)
