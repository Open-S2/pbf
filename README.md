<h1 style="text-align: center;">
<div align="center">pbf</div>
</h1>

<p align="center">
  <a href="https://img.shields.io/github/actions/workflow/status/Open-S2/pbf/test.yml?logo=github">
    <img src="https://img.shields.io/github/actions/workflow/status/Open-S2/pbf/test.yml?logo=github" alt="GitHub Actions Workflow Status">
  </a>
  <a href="https://npmjs.org/package/pbf-ts">
    <img src="https://img.shields.io/npm/v/pbf-ts.svg?logo=npm&logoColor=white" alt="npm">
  </a>
  <a href="https://crates.io/crates/pbf">
    <img src="https://img.shields.io/crates/v/pbf.svg?logo=rust&logoColor=white" alt="crate">
  </a>
  <a href="https://bundlejs.com/?q=pbf-ts&treeshake=%5B%7B+PbfReader+%7D%5D">
    <img src="https://img.shields.io/bundlejs/size/pbf-ts?exports=PbfReader" alt="bundle">
  </a>
  <a href="https://www.npmjs.com/package/pbf-ts">
    <img src="https://img.shields.io/npm/dm/pbf-ts.svg" alt="downloads">
  </a>
  <a href="https://open-s2.github.io/pbf/">
    <img src="https://img.shields.io/badge/docs-typescript-yellow.svg" alt="docs-ts">
  </a>
  <a href="https://docs.rs/pbf">
    <img src="https://img.shields.io/badge/docs-rust-yellow.svg" alt="docs-rust">
  </a>
  <a href="https://coveralls.io/github/Open-S2/pbf?branch=master">
    <img src="https://coveralls.io/repos/github/Open-S2/pbf/badge.svg?branch=master" alt="code-coverage">
  </a>
  <a href="https://discord.opens2.com">
    <img src="https://img.shields.io/discord/953563031701426206?logo=discord&logoColor=white" alt="Discord">
  </a>
</p>

## About

This module implements the [Protocol Buffer Format](https://protobuf.dev/) in a light weight, minimalistic, and efficient way.

The `pbf` Rust crate provides functionalities to read and write Protocol Buffers (protobuf) messages. This crate is a 0 dependency package that uses `no_std` and is intended to be used in embedded systems and WASM applications. The crate is designed to be small and efficient, with the cost of some features and flexibility. It is up to the user to create the necessary data structures and implement the `ProtoRead` and `ProtoWrite` traits in order to use it effectively.

## Usage

### Typescript

This is a low-level, fast, ultra-lightweight typescript library for decoding and encoding protocol buffers. It was ported from the [pbf](https://github.com/mapbox/pbf) package.

Install the package:

```bash
# bun
bun add pbf-ts
# npm
npm install pbf-ts
# pnpm
pnpm add pbf-ts
# yarn
yarn add pbf-ts
# deno
deno install pbf-ts
```

### Typescript Examples

```ts
import { readFileSync } from 'fs';
import { Pbf } from 'pbf-ts';

// Reading:
const pbf = new Pbf(readFileSync(path));

// Writing:
const pbf = new Pbf();
pbf.writeVarintField(1, 1);
// ...
const result = pbf.commit();
```

If you want to reduce build size and know you're only reading data, not writing to it, use the `PbfReader` class:

```ts
import { readFileSync } from 'fs';
import { PbfReader } from 'pbf-ts';

const pbf = new PbfReader(readFileSync(path));
// ...
```

More complex example:

```ts
/** Building a class to test with. */
class Test {
    a = 0;
    b = 0;
    c = 0;
    /**
     * @param pbf - the Protobuf object to read from
     * @param end - the position to stop at
     */
    constructor(pbf: Protobuf, end = 0) {
        pbf.readFields(Test.read, this, end);
    }
    /**
     * @param t - the test object to write.
     * @param pbf - the Protobuf object to write to.
     */
    static writeMessage(t: Test, pbf: Protobuf): void {
        pbf.writeVarintField(1, t.a);
        pbf.writeFloatField(2, t.b);
        pbf.writeSVarintField(3, t.c);
    }

    /**
     * @param tag - the tag to read.
     * @param test - the test to modify
     * @param pbf - the Protobuf object to read from
     */
    static read(tag: number, test: Test, pbf: Protobuf): void {
        if (tag === 1) test.a = pbf.readVarint();
        else if (tag === 2) test.b = pbf.readFloat();
        else if (tag === 3) test.c = pbf.readSVarint();
        else throw new Error(`Unexpected tag: ${tag}`);
    }

    /**
     * @returns - a new test object
     */
    static newTest(): Test {
        return { a: 1, b: 2.2, c: -3 } as Test;
    }

    /**
     * @returns - a new default test object
     */
    static newTestDefault(): Test {
        return { a: 0, b: 0, c: 0 } as Test;
    }
}

// Writing the message
const pbf = new Protobuf();
const t = Test.newTest();
pbf.writeMessage(5, Test.writeMessage, t);
const data = pbf.commit();
expect(data).toEqual(new Uint8Array([42, 9, 8, 1, 21, 205, 204, 12, 64, 24, 5]));

// Reading the message
const pbf2 = new Protobuf(data);
expect(pbf2.readTag()).toEqual({ tag: 5, type: Protobuf.Bytes });
const t2 = new Test(pbf2, pbf2.readVarint() + pbf2.pos);
expect(t2).toEqual({ a: 1, b: 2.200000047683716, c: -3 } as Test);
```

### Rust

Install the package:

```bash
# cargo
cargo install pbf
```

or add the following to your `Cargo.toml`:

```toml
[dependencies]
pbf = "0.3"
```

### Rust Examples

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

// write the protobuf message
let mut pb = Protobuf::new();
let msg = TestMessage::new(1, "hello");
// top level proto messages usually write fields, but inner messages use `write_message`
pb.write_fields(&msg);

// take the data as a Vec<u8>
let bytes = pb.take();

// Let's put it back into a protobuffer for reading
let mut pb = Protobuf::from_input(bytes);
let mut msg = TestMessage::default();
pb.read_fields(&mut msg, None);
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
