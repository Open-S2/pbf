# pbf-rs

The `pbf` Rust crate provides functionalities to read and write Protocol Buffers (protobuf) messages. This crate is a 0 dependency package that uses `no_std` and is intended to be used in embedded systems and WASM applications. The crate is designed to be small and efficient, with the cost of some features and flexibility. It is up to the user to create the necessary data structures and implement the `ProtoRead` and `ProtoWrite` traits in order to use it effectively.

## Development

### Requirements

The `bacon coverage` tool is used to generate the coverage report. To utilize the [pycobertura](https://pypi.org/project/pycobertura/) package for a prettier coverage report, install it using the following command:

```bash
pip install pycobertura
```

### Running Tests

To run the tests, use the following command:

```bash
cargo test
# bacon
bacon test
```

### Generating Coverage Report

To generate the coverage report, use the following command:

```bash
cargo tarpaulin
# bacon
bacon coverage # or type `l` inside the tool
```
