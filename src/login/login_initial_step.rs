use leptos::*;
use crate::CreateWalletStep;
use crate::core::NetworkType;
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_burn::LatestBurn;
use crate::pages::pixel_view::LazyPixelView;

#[component]
pub fn InitialStep(
    set_current_step: WriteSignal<CreateWalletStep>,
    selected_network: RwSignal<NetworkType>,
) -> impl IntoView {
    let (latest_burn, set_latest_burn) = create_signal(Option::<LatestBurn>::None);
    
    // Fetch latest burn on component mount
    create_effect(move |_| {
        spawn_local(async move {
            match RpcConnection::get_latest_burn().await {
                Ok(Some(mut burn)) => {
                    log::info!("Loaded latest {} burn by {}", burn.burn_type, burn.user_pubkey);
                    
                    // For chat burns without profile info, fetch the user's profile from mainnet
                    if (burn.burn_type == "chat_burn" || burn.burn_type == "chat_create") && burn.username.is_none() && !burn.user_pubkey.is_empty() {
                        let mainnet_rpc = "https://rpc.mainnet.x1.xyz";
                        let rpc = RpcConnection::with_endpoint(mainnet_rpc);
                        
                        match rpc.get_profile_mainnet(&burn.user_pubkey).await {
                            Ok(Some(profile)) => {
                                burn.username = Some(profile.username);
                                burn.image = Some(profile.image);
                                log::info!("Fetched profile for chat burn user");
                            }
                            Ok(None) => {
                                log::info!("No profile found for chat burn user");
                            }
                            Err(e) => {
                                log::warn!("Failed to fetch profile for chat burn user: {}", e);
                            }
                        }
                    }
                    
                    set_latest_burn.set(Some(burn));
                }
                Ok(None) => {
                    log::info!("No recent burns found");
                }
                Err(e) => {
                    log::warn!("Failed to fetch latest burn: {}", e);
                }
            }
        });
    });
    
    view! {
        <>
        // Latest burn card (outside login container)
        {move || {
            if let Some(burn) = latest_burn.get() {
                view! {
                    <div class="latest-burn-card-external">
                        <div class="latest-burn-content-external">
                            {if let Some(ref image) = burn.image {
                                view! {
                                    <div class="burn-avatar">
                                        <LazyPixelView
                                            art={image.clone()}
                                            size=64
                                        />
                                    </div>
                                }.into_view()
                            } else {
                                view! { <></> }.into_view()
                            }}
                            <div class="burn-info">
                                <div class="burn-header-line">
                                    <span class="burn-label">"Latest Burn"</span>
                                </div>
                                {if let Some(ref username) = burn.username {
                                    view! {
                                        <div class="burn-username">{username.clone()}</div>
                                    }.into_view()
                                } else {
                                    // Show shortened address if no username (e.g., for chat burns)
                                    let addr = burn.user_pubkey.clone();
                                    let short_addr = if addr.len() > 12 {
                                        format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
                                    } else {
                                        addr
                                    };
                                    view! {
                                        <div class="burn-username">{short_addr}</div>
                                    }.into_view()
                                }}
                                {if let Some(ref desc) = burn.description {
                                    view! {
                                        <div class="burn-description">{desc.clone()}</div>
                                    }.into_view()
                                } else {
                                    view! { <></> }.into_view()
                                }}
                            </div>
                            <div class="burn-amount-corner">
                                <i class="fas fa-fire-alt"></i>
                                " "
                                {format!("{}", burn.burn_amount)}
                                " MEMO"
                            </div>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! { <></> }.into_view()
            }
        }}
        
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
        </>
    }
}