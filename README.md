# Project Clean Tools [![Rust][badge]][rust]

Project clean tools supports `rust`, `golang`, `maven`, `gradle` projects out of the box.
But you could custom your own clean command via [custom config file](#custom-config-file): `.cleanrc`.

## Install

### Install From [crates-io](https://crates.io/crates/clean-rs)
```bash
cargo install clean-rs
```

### Install Manually
```bash
git clone https://github.com/holi-java/clean.git
cargo install --path clean
```


## Usage 

```bash
# Clean current directory
clean

# Clean the specified directory
clean /home/projects
```

## Custom Config File

Add `.cleanrc` config file under your home directory as below:

```none
# Directory ends with slash will remove the whole directory
node_modules/

# Using custom command to run clean
pom.xml = mvn -B --offline clean
```

[badge]: https://github.com/holi-java/clean/actions/workflows/rust.yml/badge.svg?branch=main
[rust]: https://github.com/holi-java/clean/actions/workflows/rust.yml
