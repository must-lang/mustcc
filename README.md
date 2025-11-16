# Must compiler

To install:

`$ cargo install --path .`

To run example:

`$ mustcc examples/001`

and then link using your system linker:

`$ cc examples/001/output.o`

To see available flags:

`$ mustcc --help`

### Disclaimer

Note that most of the language features are not yet implemented in the backend,
for testing the typechecker you can use `-t` switch.
