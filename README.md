# picorne firmware

Based on the [keyberon](https://github.com/TeXitoi/keyberon) library to create keyboard firmwares.

## Customising the layout
Check out `src/layout.rs`.
Just modify the contents of the `layout!` block according to the following documentation.

For a letter key, just write the letter in caps. For a symbol or number, just write it. In the case of special characters, wrap them in single quotes.

Write `n` to do nothing.

Write `t` for a transparent key that goes into the below layer.

Write a number in brackets to transition to that layer.

Write multiple actions in square brackets to do them all.

To reset the pico or boot it into bootsel mode, write `{Custom(Reset)}` or `{Custom(Bootsel)}`.

A layer is enclosed by curly braces, and each row is enclosed by square brackets. Layers are zero-indexed and it defaults to layer 0.

See https://github.com/TeXitoi/keyberon/pull/54 for more details.

## Building
With [rust installed](https://rustup.rs):
```sh
rustup target install thumbv6m-none-eabi
cargo install flip-link
cargo install elf2uf2-rs
cargo build --release
elf2uf2-rs target/thumbv6m-none-eabi/release/picorne
```
Then find your built firmware at target/thumbv6m-none-eabi/release/picorne.uf2