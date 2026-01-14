# disty

disty is a quick CLI for getting an idea of the distribution of a list of numbers.

disty is a rewrite of [Michael Knyszek's distx](https://github.com/mknyszek/toolbox/tree/main/cmd/distx), which is itself an extension of [Austin Clements' dist](https://github.com/aclements/go-moremath/tree/master/cmd/dist). I use distx for all sorts of quick numeric checks, from checking the distribution of database segments to analyzing request latency. I ran into performance issues for processing large lists (>1m records) which inspired the rewrite.

Compared to distx, this version:

- Is very fast: ~55x faster in local testing. This boils down to parallelizing the KDE plotting, using `mmap` to parallelize parsing the number list, and reducing unnecessary copying.
    ```bash
    $ seq 1 10000000 | rg 1 > /tmp/large_seq
    $ wc -l /tmp/large_seq
    5217032 /tmp/large_seq
    $ hyperfine --warmup 3 'distx /tmp/large_seq' 'disty /tmp/large_seq'
    Benchmark 1: distx /tmp/large_seq
      Time (mean ± σ):     10.750 s ±  0.625 s    [User: 10.711 s, System: 0.471 s]
      Range (min … max):    9.546 s … 11.438 s    10 runs

    Benchmark 2: disty /tmp/large_seq
      Time (mean ± σ):     195.6 ms ±   1.9 ms    [User: 1757.8 ms, System: 153.9 ms]
      Range (min … max):   191.3 ms … 199.4 ms    15 runs

    Summary
      disty /tmp/large_seq ran
       54.95 ± 3.24 times faster than distx /tmp/large_seq    
    ```

- Has marginally better plotting, which mostly comes down to setting a higher resolution than distx uses by default. ![Example screenshot](media/image.png)

- Has less features. I haven't ported the output options or alternative plotting (CDFs), because I don't really use them. 

## Installing

Install via Cargo.

```sh
# From Crates.io
$ cargo install disty-cli
# From source
$ cargo install --path .
```

## Usage

```bash
$ disty --help
Summarizes numerical distributions

Usage: disty [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file (stdin if not specified)

Options:
  -u, --unit <UNIT>  Input unit [possible values: ns, us, ms, s, B, KB, MB, GB, TB, PB, KiB, MiB, GiB, TiB, PiB]
  -f, --fmt <FMT>    Output format [possible values: float, hex, time, bytes]
      --no-plot      Skip KDE plotting
  -h, --help         Print help
  -V, --version      Print version
```

## Development

### Running Tests

```bash
cargo test
```

### Running Benchmarks

The project includes criterion benchmarks for parsing, statistics computation, and KDE evaluation:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench disty_bench -- parsing
cargo bench --bench disty_bench -- stats
cargo bench --bench disty_bench -- kde
cargo bench --bench disty_bench -- full_pipeline

# Run benchmarks with different sample sizes
cargo bench --bench disty_bench -- "1000000"
```

Benchmarks test with various input sizes (1K, 10K, 100K, 1M elements) to understand performance characteristics at different scales.
