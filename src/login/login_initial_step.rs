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
            <h1 class="app-title">"MEMO Engraves Memories Onchain"</h1>
            
            // Network selector
            <div class="network-selector-container">
                <label class="network-label">"Select Network:"</label>
                <div class="network-options">
                    <button
                        class=move || if selected_network.get() == NetworkType::Testnet {
                            "network-option active network-testnet"
                        } else {
                            "network-option network-testnet"
                        }
                        on:click=move |_| selected_network.set(NetworkType::Testnet)
                    >
                        <div class="network-option-header">
                            <span class="network-name">"Testnet"</span>
                            <span class="network-badge network-badge-testnet">"DEV/TEST"</span>
                        </div>
                        <div class="network-description">
                            {NetworkType::Testnet.description()}
                        </div>
                    </button>
                    
                    <button
                        class=move || if selected_network.get() == NetworkType::ProdStaging {
                            "network-option active network-staging"
                        } else {
                            "network-option network-staging"
                        }
                        on:click=move |_| selected_network.set(NetworkType::ProdStaging)
                    >
                        <div class="network-option-header">
                            <span class="network-name">"Prod Staging"</span>
                            <span class="network-badge network-badge-staging">"STAGING"</span>
                        </div>
                        <div class="network-description">
                            {NetworkType::ProdStaging.description()}
                        </div>
                    </button>
                    
                    <button
                        class=move || if selected_network.get() == NetworkType::Mainnet {
                            "network-option active network-mainnet"
                        } else {
                            "network-option network-mainnet"
                        }
                        on:click=move |_| selected_network.set(NetworkType::Mainnet)
                    >
                        <div class="network-option-header">
                            <span class="network-name">"Mainnet"</span>
                            <span class="network-badge network-badge-mainnet">"PROD"</span>
                        </div>
                        <div class="network-description">
                            {NetworkType::Mainnet.description()}
                        </div>
                    </button>
                </div>
            </div>
            
            <div class="button-group">
                <button 
                    class="wallet-btn new-wallet" 
                    on:click=move |_| set_current_step.set(CreateWalletStep::ShowMnemonic(String::new()))
                >
                    "New Wallet"
                </button>
                <button
                    class="wallet-btn import-wallet"
                    on:click=move |_| {
                        set_current_step.set(CreateWalletStep::ImportMnemonic);
                    }
                >
                    "Import Wallet"
                </button>
            </div>
        </div>
    }
}