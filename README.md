# Scout

An experimental static analysis tool to help find malicious or harmful code in Python packages.

## Project Setup

The project requires at least Rust `v1.65.0` to run. The stable version is recommended. You can use the [rustup installer](https://rustup.rs/) to get rust on your machine.

First clone the repository:
```
git clone https://github.com/Syntox32/scout/
```

For development use the command:
```
$ cargo build 
OR 
$ cargo run
```

For performance testing and gathering metrics it is recommended that the program is built in release mode:
```
$ cargo build --release
OR 
$ cargo run --release
```

## Configuration

### Logging

The `scout` library uses the [env_logger](https://docs.rs/env_logger/latest/env_logger/) crate for configuring logging.

To enable debug logging, insert the following before your cargo command:
```
RUST_LOG=<log level> cargo run [...]
```

Relevant log levels might be `error`, `warn`, `info`, `debug`, `trace`, or `off`. For more information see the `env_logger` documentation.

## Usage examples

Paths in the examples are formatted to work on Windows 10.

### Textual report for a given file

```
$ cargo run -- --file ./examples/files/test-obfuscated-example.py --threshold 0.3 --all true
```

### Raw JSON output for a given file

```
$ cargo run -- --file ./examples/files/test-obfuscated-example.py --threshold 0.3 --json true --all true
```

### Formatted JSON output for a given file

```
$ cargo run -- --file ./examples/files/test-obfuscated-example.py --threshold 0.3 --json true --all true | python -m json.tool
```

## Configuring Matplotlib for graph output

To plot the field data using Python you should have a recent version of `Python 3` and `matplotlib` installed.

After this you can pipe data from `scout` to the `plot.py` helper script. It's important you use the `--fields true` and `--json true` flag on `scout` or else the required data will not be included in the JSON output:

```
cargo run -- --file ./examples/files/test-obfuscated-example.py --threshold 0.3 --all true --json true --fields true | python scripts/plot.py -T 0.1
```

The `-T` flag on the script draws a horizontal line at that threshold, which can be useful for debugging.

## Known issues

Hotspots are chunked slightly differently accross several runs. This might be an issue with the order bulletins are added and calculated, or is perhaps caused by floating point errors.

## Licence

The project is currently under a GPLv2 licence. This might change to a more Rust canonical dual licencing in later versions.