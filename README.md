# Canvas

Canvas is a Rust library for building form, single-line input, and textarea
terminal user interfaces.

## First class support
This crate has first class support from tui-pages. It implements defaults for this crate out of the box. Visit:
https://tui-pages.farmeris.sk/

## Running Examples

```bash
cargo run --example canvas_keybindings       --features "gui keybindings cursor-style"
cargo run --example form_vim                 --features "gui keybindings cursor-style"
cargo run --example canvas_cursor_auto       --features "gui cursor-style"
cargo run --example computed_fields          --features "gui computed"

cargo run --example textarea_vim             --features "gui cursor-style textarea textmode-vim"
cargo run --example textarea_normal          --features "gui cursor-style textarea textmode-normal commandline"
cargo run --example textarea_syntax          --features "gui cursor-style textarea textmode-normal syntect"
cargo run --example textarea_vim_minimal     --features "textarea keybindings cursor-style commandline"
cargo run --example textarea_helix_minimal   --features "textarea keybindings cursor-style commandline"
# VSCode is modeless, so it needs --no-default-features (textmode-normal conflicts with the default textmode-vim).
# Include `clipboard` so Ctrl+C/Ctrl+X mirror to the OS clipboard (it's a default feature dropped by --no-default-features).
cargo run --example textarea_vscode_minimal  --no-default-features --features "gui textarea keybindings cursor-style commandline textmode-normal clipboard"

cargo run --example textinput_normal         --features "gui cursor-style textinput textmode-normal"
cargo run --example undo_redo                --features "gui cursor-style textinput"

cargo run --example validation_1             --features "gui validation cursor-style"
cargo run --example validation_2             --features "gui validation cursor-style"
cargo run --example validation_3             --features "gui validation cursor-style"
cargo run --example validation_4             --features "gui validation cursor-style"
cargo run --example validation_5             --features "gui validation cursor-style"

cargo run --example suggestions              --features "suggestions gui cursor-style"
cargo run --example suggestions2             --features "suggestions gui cursor-style"
```

## License

Licensed under the MIT License.
