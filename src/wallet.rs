//! Browser wallets injected by extensions: X1 Wallet (`window.x1`) and Backpack (`window.backpack`).
//! The app never sees a private key: it builds an unsigned transaction and the wallet signs it.

use js_sys::{Function, Promise, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletKind {
    X1,
    Backpack,
}

fn get(obj: &JsValue, key: &str) -> Option<JsValue> {
    Reflect::get(obj, &JsValue::from_str(key)).ok().filter(|v| !v.is_null() && !v.is_undefined())
}

fn js_error(e: JsValue) -> String {
    get(&e, "message")
        .and_then(|m| m.as_string())
        .or_else(|| e.as_string())
        .unwrap_or_else(|| "Request rejected".to_string())
}

async fn call(obj: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, String> {
    let func: Function = get(obj, method)
        .and_then(|f| f.dyn_into().ok())
        .ok_or_else(|| format!("Wallet has no {}()", method))?;
    let result = match args {
        [] => func.call0(obj),
        [a] => func.call1(obj, a),
        _ => unreachable!(),
    }
    .map_err(js_error)?;
    if result.has_type::<Promise>() {
        JsFuture::from(Promise::from(result)).await.map_err(js_error)
    } else {
        Ok(result)
    }
}

fn to_text(v: &JsValue) -> Option<String> {
    v.as_string().or_else(|| {
        let f: Function = get(v, "toString")?.dyn_into().ok()?;
        f.call0(v).ok()?.as_string()
    })
}

fn bytes_to_b64(v: &JsValue) -> Option<String> {
    v.dyn_ref::<Uint8Array>().map(|a| base64::encode(a.to_vec()))
}

impl WalletKind {
    pub const ALL: [WalletKind; 2] = [WalletKind::X1, WalletKind::Backpack];

    pub fn name(self) -> &'static str {
        match self {
            WalletKind::X1 => "X1 Wallet",
            WalletKind::Backpack => "Backpack",
        }
    }

    pub fn install_url(self) -> &'static str {
        match self {
            WalletKind::X1 => "https://x1.xyz",
            WalletKind::Backpack => "https://backpack.app",
        }
    }

    fn provider(self) -> Option<JsValue> {
        let window: JsValue = web_sys::window()?.into();
        get(&window, match self {
            WalletKind::X1 => "x1",
            WalletKind::Backpack => "backpack",
        })
    }

    pub fn is_installed(self) -> bool {
        self.provider().is_some()
    }

    /// Ask the wallet to connect; returns the address
    pub async fn connect(self) -> Result<String, String> {
        let provider = self.provider().ok_or_else(|| format!("{} is not installed", self.name()))?;
        let result = call(&provider, "connect", &[]).await?;
        get(&result, "publicKey")
            .and_then(|k| to_text(&k))
            .or_else(|| result.as_string())
            .or_else(|| get(&provider, "publicKey").and_then(|k| to_text(&k)))
            .ok_or_else(|| "The wallet returned no address".to_string())
    }

    pub async fn disconnect(self) {
        if let Some(provider) = self.provider() {
            let _ = call(&provider, "disconnect", &[]).await;
        }
    }

    /// Sign a transaction (bincode, base64); returns the signed transaction as base64
    pub async fn sign(self, tx_b64: &str) -> Result<String, String> {
        let arg = JsValue::from_str(tx_b64);
        let result = match self {
            WalletKind::X1 => {
                let provider = self.provider().ok_or("X1 Wallet is not installed")?;
                call(&provider, "signTransaction", &[arg]).await?
            }
            WalletKind::Backpack => {
                // index.html deserializes it with web3.js, which Backpack expects
                let window: JsValue = web_sys::window().ok_or("No window")?.into();
                let helper = get(&window, "MemoBackpack").ok_or("Backpack helper missing")?;
                call(&helper, "signTransaction", &[arg]).await?
            }
        };
        // X1 Wallet has returned several shapes over time; accept each of them
        result
            .as_string()
            .or_else(|| bytes_to_b64(&result))
            .or_else(|| {
                let bytes = get(&result, "_signedBytes")?;
                // A plain object keyed "0", "1", ... (no length property)
                let v: Vec<u8> = (0u32..)
                    .map_while(|i| Reflect::get(&bytes, &JsValue::from(i)).ok()?.as_f64().map(|b| b as u8))
                    .collect();
                (!v.is_empty()).then(|| base64::encode(v))
            })
            .or_else(|| {
                ["signedTransaction", "transaction"].iter().find_map(|k| {
                    let v = get(&result, k)?;
                    v.as_string().or_else(|| bytes_to_b64(&v))
                })
            })
            .ok_or_else(|| "The wallet returned an unknown signed transaction".to_string())
    }
}
