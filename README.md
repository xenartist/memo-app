# Tauri + Leptos

This template should help get you started developing with Tauri and Leptos.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).


# Unit Test
* Normal Unit Test
```
cargo test
```

* RPC Specific Unit Test

Headless Mode in Bash
```
set RUST_LOG=error,tiny_http=off && wasm-pack test --chrome --headless
```

Headless Mode in Powershell
```
$env:RUST_LOG="error,tiny_http=off"; wasm-pack test --chrome --headless
```

Interactive Mode in Browser
```
wasm-pack test --chrome
```
