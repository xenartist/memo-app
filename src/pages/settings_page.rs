use leptos::*;
use crate::core::network_config::{try_get_network_config, NetworkType};
use crate::core::settings::{RpcSelection, UserSettings, load_settings_for_network, save_settings_for_network};
use std::time::Duration;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let Some(network_config) = try_get_network_config() else {
        return view! {
            <div class="settings-page">
                <h2>"Settings"</h2>
                <div class="settings-section">
                    <h3>"RPC Configuration"</h3>
                    <p class="settings-warning">
                        "Network is not initialized. Please log in again to configure RPC settings."
                    </p>
                </div>
            </div>
        };
    };

    let network_type = network_config.network_type;
    let network_display_name = network_type.display_name();
    let network_description = network_type.description();
    let default_rpc_url = network_config
        .rpc_endpoints
        .first()
        .map(|url| url.to_string())
        .unwrap_or_default();
    let default_rpc_label = if default_rpc_url.is_empty() {
        "Default".to_string()
    } else {
        format!("Default ({default_rpc_url})")
    };
    let default_rpc_for_current = default_rpc_url.clone();
    let network_style_class = match network_type {
        NetworkType::Testnet => "settings-network-testnet",
        NetworkType::ProdStaging => "settings-network-staging",
        NetworkType::Mainnet => "settings-network-mainnet",
    };
    let base_section_class = format!("settings-section settings-section-network {}", network_style_class);
    let rpc_section_classes = format!("{base_section_class} settings-section-rpc");
    let compute_section_classes = format!("{base_section_class} settings-section-compute");

    let stored = load_settings_for_network(network_type);

    let initial_rpc_selection = stored
        .as_ref()
        .map(|s| {
            if matches!(s.rpc_selection, RpcSelection::Custom) && s.custom_rpc_url.trim().is_empty() {
                RpcSelection::Default
            } else {
                s.rpc_selection.clone()
            }
        })
        .unwrap_or(RpcSelection::Default);

    let initial_custom_rpc = stored
        .as_ref()
        .map(|s| s.custom_rpc_url.clone())
        .unwrap_or_default();

    let initial_compute_buffer = stored
        .as_ref()
        .map(|s| s.compute_unit_buffer_percentage.min(100))
        .unwrap_or(1);

    let initial_compute_price = stored
        .as_ref()
        .map(|s| s.compute_unit_price_micro_lamports)
        .unwrap_or(0);

    let (rpc_selection, set_rpc_selection) = create_signal(initial_rpc_selection);
    let (custom_rpc_url, set_custom_rpc_url) = create_signal(initial_custom_rpc);
    let (compute_unit_buffer_percentage, set_compute_unit_buffer_percentage) =
        create_signal(initial_compute_buffer);
    let (compute_unit_price_micro_lamports, set_compute_unit_price_micro_lamports) =
        create_signal(initial_compute_price);
    let (save_feedback, set_save_feedback) = create_signal(Option::<String>::None);

    let current_rpc_url = move || match rpc_selection.get() {
        RpcSelection::Default => default_rpc_for_current.clone(),
        RpcSelection::Custom => {
            let custom = custom_rpc_url.get();
            if custom.trim().is_empty() {
                default_rpc_for_current.clone()
            } else {
                custom
            }
        }
    };

    let save_settings_action = {
        move |_| {
            let settings = UserSettings {
                rpc_selection: rpc_selection.get_untracked(),
                custom_rpc_url: custom_rpc_url.get_untracked(),
                compute_unit_buffer_percentage: compute_unit_buffer_percentage.get_untracked(),
                compute_unit_price_micro_lamports: compute_unit_price_micro_lamports.get_untracked(),
            };

            match save_settings_for_network(network_type, &settings) {
                Ok(_) => {
                    let message = format!("{network_display_name} settings saved to browser storage.");
                    set_save_feedback.set(Some(message));
                    set_timeout(
                        {
                            let set_save_feedback = set_save_feedback.clone();
                            move || set_save_feedback.set(None)
                        },
                        Duration::from_secs(3),
                    );
                }
                Err(err) => {
                    log::error!("Failed to save settings: {err}");
                    set_save_feedback.set(Some("Failed to save settings.".to_string()));
                    set_timeout(
                        {
                            let set_save_feedback = set_save_feedback.clone();
                            move || set_save_feedback.set(None)
                        },
                        Duration::from_secs(3),
                    );
                }
            }
        }
    };

    view! {
        <div class="settings-page">
            <h2>"Settings"</h2>
            
            <div class={rpc_section_classes.clone()}>
                <h3>"RPC Configuration"</h3>
                
                <div class="rpc-settings">
                    <div class="network-group">
                        <h4>{format!("{network_display_name} RPC")}</h4>
                        <p class="network-description">{network_description}</p>

                        <div class="radio-option">
                            <input 
                                type="radio"
                                id="network-default"
                                name="rpc-network"
                                checked=move || rpc_selection.get() == RpcSelection::Default
                                on:change=move |_| set_rpc_selection.set(RpcSelection::Default)
                            />
                            <label for="network-default">{default_rpc_label.clone()}</label>
                        </div>
                        
                        <div class="radio-option">
                            <input 
                                type="radio"
                                id="network-custom"
                                name="rpc-network"
                                checked=move || rpc_selection.get() == RpcSelection::Custom
                                on:change=move |_| set_rpc_selection.set(RpcSelection::Custom)
                            />
                            <label for="network-custom">"Custom"</label>
                            <input 
                                type="text"
                                class="custom-rpc-input"
                                placeholder={format!("Enter custom {network_display_name} RPC URL")}
                                prop:value=move || custom_rpc_url.get()
                                on:input=move |ev| {
                                    set_custom_rpc_url.set(event_target_value(&ev));
                                }
                                disabled=move || rpc_selection.get() != RpcSelection::Custom
                            />
                        </div>
                    </div>

                    <div class="current-rpc">
                        <h4>"Current RPC URL:"</h4>
                        <div class="rpc-url">{current_rpc_url}</div>
                    </div>
                </div>
            </div>

            <div class={compute_section_classes.clone()}>
                <h3>"Compute Unit"</h3>
                <div class="form-field">
                    <label for="compute-buffer">"Compute Unit Buffer (%)"</label>
                    <input
                        type="number"
                        id="compute-buffer"
                        min="0"
                        max="100"
                        step="1"
                        prop:value=move || compute_unit_buffer_percentage.get().to_string()
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            let parsed = value.trim().parse::<u32>().unwrap_or(0).min(100);
                            set_compute_unit_buffer_percentage.set(parsed);
                        }
                    />
                    <small class="field-help">
                        "Buffer percentage added to simulated CU. Final CU = simulated × (1 + buffer%). Default: 0%"
                    </small>
                </div>

                <div class="form-field">
                    <label for="compute-price">"Compute Unit Price (micro-lamports)"</label>
                    <input
                        type="number"
                        id="compute-price"
                        min="0"
                        step="1"
                        prop:value=move || compute_unit_price_micro_lamports.get().to_string()
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            let parsed = value.trim().parse::<u64>().unwrap_or(0);
                            set_compute_unit_price_micro_lamports.set(parsed);
                        }
                    />
                    <small class="field-help">"Priority fee per compute unit. Higher values = faster processing. Default: 0 (no priority fee)"</small>
                </div>
            </div>

            <div class="settings-actions">
                <button
                    class="settings-btn save-btn"
                    type="button"
                    on:click=save_settings_action
                >
                    <i class="fas fa-save"></i>
                    <span>"Save"</span>
                </button>
            </div>

            <Show when=move || save_feedback.get().is_some()>
                <p class="save-feedback">{move || save_feedback.get().unwrap_or_default()}</p>
            </Show>
        </div>
    }
} 