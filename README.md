# MEMO app

Web client for the MEMO protocol on X1. Rust + [Leptos](https://leptos.dev), built with [Trunk](https://trunkrs.dev).

v2 (this folder) is deliberately small: one MEMO Token page that explains MEMO, shows the emission curve
and the current reward, and mints with an external wallet (X1 Wallet or Backpack). The app never
holds keys; the wallet signs every transaction.

```bash
cargo install trunk
rustup target add wasm32-unknown-unknown
trunk serve            # http://127.0.0.1:1420
trunk build --release  # output in dist/
```

`src/`:
- `curve.rs`: the memo-mint emission curve (same arithmetic as the contract)
- `rpc.rs`: X1 JSON-RPC calls and the mint transaction
- `wallet.rs`: X1 Wallet / Backpack connect and sign
- `network.rs`: Mainnet / Testnet settings
- `app.rs`: layout and the MEMO Token page

`v1/` is the previous app (embedded mnemonic wallet, auto-mint, chat, forum, blog, projects,
profiles, trade). It is kept for reference and still builds on its own: `cd v1 && trunk build`.
