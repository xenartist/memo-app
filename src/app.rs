use leptos::prelude::*;
use leptos::logging::log;
use leptos::ev;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    // Handler functions for the buttons
    let on_new_wallet = move |_: ev::MouseEvent| {
        // TODO: Implement new wallet creation logic
        log!("New Wallet button clicked");
    };

    let on_import_wallet = move |_: ev::MouseEvent| {
        // TODO: Implement wallet import logic
        log!("Import Wallet button clicked");
    };

    view! {
        <main class="container">
            <div class="login-container">
                <h1 class="app-title">"Memo App"</h1>
                <div class="button-group">
                    <button 
                        class="wallet-btn new-wallet" 
                        type="button"
                        on:click=on_new_wallet
                    >
                        "New Wallet"
                    </button>
                    <button 
                        class="wallet-btn import-wallet" 
                        type="button"
                        on:click=on_import_wallet
                    >
                        "Import Wallet"
                    </button>
                </div>
            </div>
        </main>
    }
}
