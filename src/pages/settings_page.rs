use leptos::*;

// rpc network enum
#[derive(Clone, PartialEq)]
enum RpcNetwork {
    TestnetDefault,
    TestnetCustom,
    MainnetDefault,
    MainnetCustom,
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    // create signal to store current selected network and custom rpc url
    let (selected_network, set_selected_network) = create_signal(RpcNetwork::TestnetDefault);
    let (custom_testnet_url, set_custom_testnet_url) = create_signal(String::new());
    let (custom_mainnet_url, set_custom_mainnet_url) = create_signal(String::new());

    // get current rpc url
    let current_rpc_url = move || {
        match selected_network.get() {
            RpcNetwork::TestnetDefault => "https://rpc-testnet.x1.wiki".to_string(),
            RpcNetwork::TestnetCustom => custom_testnet_url.get(),
            RpcNetwork::MainnetDefault => "TBD".to_string(),
            RpcNetwork::MainnetCustom => custom_mainnet_url.get(),
        }
    };

    view! {
        <div class="settings-page">
            <h2>"Settings"</h2>
            
            <div class="settings-section">
                <h3>"RPC Configuration"</h3>
                
                <div class="rpc-settings">
                    // Testnet 选项
                    <div class="network-group">
                        <h4>"Testnet"</h4>
                        <div class="radio-option">
                            <input 
                                type="radio"
                                id="testnet-default"
                                name="rpc-network"
                                checked=move || selected_network.get() == RpcNetwork::TestnetDefault
                                on:change=move |_| set_selected_network.set(RpcNetwork::TestnetDefault)
                            />
                            <label for="testnet-default">"Default (https://rpc-testnet.x1.wiki)"</label>
                        </div>
                        
                        <div class="radio-option">
                            <input 
                                type="radio"
                                id="testnet-custom"
                                name="rpc-network"
                                checked=move || selected_network.get() == RpcNetwork::TestnetCustom
                                on:change=move |_| set_selected_network.set(RpcNetwork::TestnetCustom)
                            />
                            <label for="testnet-custom">"Custom"</label>
                            <input 
                                type="text"
                                class="custom-rpc-input"
                                placeholder="Enter custom testnet RPC URL"
                                prop:value=move || custom_testnet_url.get()
                                on:input=move |ev| {
                                    set_custom_testnet_url.set(event_target_value(&ev));
                                }
                                disabled=move || selected_network.get() != RpcNetwork::TestnetCustom
                            />
                        </div>
                    </div>

                    // Mainnet 选项
                    <div class="network-group">
                        <h4>"Mainnet"</h4>
                        <div class="radio-option">
                            <input 
                                type="radio"
                                id="mainnet-default"
                                name="rpc-network"
                                checked=move || selected_network.get() == RpcNetwork::MainnetDefault
                                on:change=move |_| set_selected_network.set(RpcNetwork::MainnetDefault)
                            />
                            <label for="mainnet-default">"Default (TBD)"</label>
                        </div>
                        
                        <div class="radio-option">
                            <input 
                                type="radio"
                                id="mainnet-custom"
                                name="rpc-network"
                                checked=move || selected_network.get() == RpcNetwork::MainnetCustom
                                on:change=move |_| set_selected_network.set(RpcNetwork::MainnetCustom)
                            />
                            <label for="mainnet-custom">"Custom"</label>
                            <input 
                                type="text"
                                class="custom-rpc-input"
                                placeholder="Enter custom mainnet RPC URL"
                                prop:value=move || custom_mainnet_url.get()
                                on:input=move |ev| {
                                    set_custom_mainnet_url.set(event_target_value(&ev));
                                }
                                disabled=move || selected_network.get() != RpcNetwork::MainnetCustom
                            />
                        </div>
                    </div>

                    // show current selected rpc url
                    <div class="current-rpc">
                        <h4>"Current RPC URL:"</h4>
                        <div class="rpc-url">{current_rpc_url}</div>
                    </div>
                </div>
            </div>
        </div>
    }
} 