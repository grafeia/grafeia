# Γραφείο 

Γραφείο (greek for writing table) is an experiment in typography and editing.
It is intended to be eventually useful to authors who want to escape the dull look of modern books and to give them the tools to create beautiful art – content and layout.

## DEMO
You can [try it in your browser](https://grafeia.github.io/grafeia-wasm), if it supports WebGL2 (+EXT_color_buffer_float) and WebAssembly.

## Run it yourself

1. Fetch the code
```sh
git clone https://github.com/grafeia/grafeia/
cd grafeia
```

2. Then generate the demo file (`demo.graf` in the current directory).
```sh
cargo run --release --bin demo
```

3. Run grafeia and load the demo file
```sh
cargo run --release --bin grafeia_gui demo.graf
```
