# pbf-rs [![docs-rust][docs-rust-image]][docs-rust-url]

[docs-rust-image]: https://img.shields.io/badge/docs-rust-yellow.svg
[docs-rust-url]: https://docs.rs/pbf

The `pbf` Rust crate provides functionalities to read and write Protocol Buffers (protobuf) messages. This crate is a 0 dependency package that uses `no_std` and is intended to be used in embedded systems and WASM applications. The crate is designed to be small and efficient, with the cost of some features and flexibility. It is up to the user to create the necessary data structures and implement the `ProtoRead` and `ProtoWrite` traits in order to use it effectively.

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
pbf = "0.1"
```

## Examples

```rust
use pbf::{ProtoRead, ProtoWrite, Protobuf, Field, Type};

#[derive(Default)]
struct TestMessage {
    a: i32,
    b: String,
}
impl TestMessage {
    fn new(a: i32, b: &str) -> Self {
        TestMessage { a, b: b.to_owned() }
    }
}
impl ProtoWrite for TestMessage {
    fn write(&self, pb: &mut Protobuf) {
        pb.write_varint_field::<u64>(1, self.a as u64);
        pb.write_string_field(2, &self.b);
    }
}
impl ProtoRead for TestMessage {
    fn read(&mut self, tag: u64, pb: &mut Protobuf) {
        println!("tag: {}", tag);
        match tag {
            1 => self.a = pb.read_varint::<i32>(),
            2 => self.b = pb.read_string(),
            _ => panic!("Invalid tag"),
        }
    }
}

let mut pb = Protobuf::new();
let msg = TestMessage::new(1, "hello");
pb.write_message(1, &msg);

let bytes = pb.take();
let mut pb = Protobuf::from_input(RefCell::new(bytes));

// first read in the field for the message
let field = pb.read_field();
assert_eq!(
    field,
    Field {
        tag: 1,
        r#type: Type::Bytes
    }
);

let mut msg = TestMessage::default();
pb.read_message(&mut msg);
assert_eq!(msg.a, 1);
assert_eq!(msg.b, "hello");
```

## Development

### Requirements

You need the tool `tarpaulin` to generate the coverage report. Install it using the following command:

```bash
cargo install cargo-tarpaulin
```

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
