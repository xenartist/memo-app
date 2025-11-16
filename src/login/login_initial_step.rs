use leptos::*;
use crate::CreateWalletStep;
use crate::core::NetworkType;

#[component]
pub fn InitialStep(
    set_current_step: WriteSignal<CreateWalletStep>,
    selected_network: RwSignal<NetworkType>,
) -> impl IntoView {
    view! {
        <div class="login-container">
            // MEMO Token Logo
            <div class="logo-container">
                <img 
                    src="https://raw.githubusercontent.com/xenartist/memo-token/refs/heads/main/metadata/memo_token-logo.png" 
                    alt="MEMO Token Logo" 
                    class="memo-logo"
                />
            </div>
            
            <h1 class="app-title">"MEMO Engraves Memories Onchain"</h1>
            
            // Network selector
            <div class="network-selector-container">
                <label class="network-label" for="network-select">"Select X1 Network:"</label>
                <select
                    id="network-select"
                    class=move || match selected_network.get() {
                        NetworkType::Testnet => "network-select network-testnet",
                        NetworkType::ProdStaging => "network-select network-staging",
                        NetworkType::Mainnet => "network-select network-mainnet",
                    }
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        match value.as_str() {
                            "testnet" => selected_network.set(NetworkType::Testnet),
                            "prod-staging" => selected_network.set(NetworkType::ProdStaging),
                            "mainnet" => selected_network.set(NetworkType::Mainnet),
                            _ => {}
                        }
                    }
                >
                    <option value="mainnet" selected=move || selected_network.get() == NetworkType::Mainnet>
                        "Mainnet - Production"
                    </option>
                    <option value="prod-staging" selected=move || selected_network.get() == NetworkType::ProdStaging>
                        "Prod Staging"
                    </option>
                    <option value="testnet" selected=move || selected_network.get() == NetworkType::Testnet>
                        "Testnet - Dev/Test"
                    </option>
                </select>
                <div class="network-description-box">
                    {move || match selected_network.get() {
                        NetworkType::Testnet => view! {
                            <div class="network-description network-desc-testnet">
                                <span class="network-badge network-badge-testnet">"DEV/TEST"</span>
                                <span>{NetworkType::Testnet.description()}</span>
                            </div>
                        },
                        NetworkType::ProdStaging => view! {
                            <div class="network-description network-desc-staging">
                                <span class="network-badge network-badge-staging">"STAGING"</span>
                                <span>{NetworkType::ProdStaging.description()}</span>
                            </div>
                        },
                        NetworkType::Mainnet => view! {
                            <div class="network-description network-desc-mainnet">
                                <span class="network-badge network-badge-mainnet">"PRODUCTION"</span>
                                <span>{NetworkType::Mainnet.description()}</span>
                            </div>
                        },
                    }}
                </div>
            </div>
            
            <div class="wallet-options-grid">
                <button 
                    class="wallet-btn new-wallet" 
                    on:click=move |_| set_current_step.set(CreateWalletStep::ShowMnemonic(String::new()))
                >
                    <i class="fas fa-plus-circle"></i>
                    "New Wallet"
                </button>
                <button
                    class="wallet-btn import-wallet"
                    on:click=move |_| {
                        set_current_step.set(CreateWalletStep::ImportMnemonic);
                    }
                >
                    <i class="fas fa-file-import"></i>
                    "Import Wallet"
                </button>
            </div>
            
            <div class="divider">
                <span class="divider-text">"OR"</span>
            </div>
            
            <div class="button-group">
                <button
                    class="wallet-btn backpack-wallet"
                    on:click=move |_| {
                        set_current_step.set(CreateWalletStep::BackpackConnect);
                    }
                >
                    <span class="backpack-icon">"🎒"</span>
                    " Connect Backpack Wallet"
                </button>
            </div>
        </div>
    }
}