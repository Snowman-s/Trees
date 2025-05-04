# Trees-lang

`trees-lang` is the official implementation of the Trees programming language, currently supporting only parsing features. It provides tools to parse and analyze Trees code, including splitting code into characters, finding blocks, and connecting blocks.

## Features

- Parse Trees code into structured blocks.
- Support for different character width modes (Mono, Half, Full).
- Error handling for compilation issues like dangling edges or multiple start blocks.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
trees-lang = "0.2.5"
```

## Usage

Here is an example of how to use `trees-lang` to parse and connect blocks in Trees code:

```rust
use trees_lang::compile::{split_code, find_blocks, connect_blocks, CompileConfig};

let code = vec![
    "    ".to_owned(),
    "    ┌───────┐".to_owned(),
    "    │ abc   │    ".to_owned(),
    "    └───┬───┘   ".to_owned(),
    "        │   ".to_owned(),
    "    ┌───┴──┐".to_owned(),
    "    │ def  │    ".to_owned(),
    "    └──────┘   ".to_owned(),
];

let splited_code = split_code(&code, &CompileConfig::DEFAULT);
let mut blocks = find_blocks(&splited_code, &CompileConfig::DEFAULT);
let head = connect_blocks(&splited_code, &mut blocks, &CompileConfig::DEFAULT).unwrap();

assert_eq!(head.proc_name, "abc".to_owned());
```

## Documentation

Comprehensive documentation is available at [docs.rs/trees-lang](https://docs.rs/trees-lang/).

There are also [wiki](https://github.com/Snowman-s/Trees/wiki) of Trees languages ... but in Japanese.

## License

This project is licensed under the [MIT License](https://github.com/Snowman-s/Trees/blob/main/LICENSE).

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests on the [GitHub repository](https://github.com/Snowman-s/Trees).

## Author

Developed by Snowman-s.
