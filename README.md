# Project Clean Tools

## Install

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

```
# Directory ends with slash will remove the whole directory
node_modules/

# Using custom command to run clean
pom.xml = mvn -B --offline clean
```
