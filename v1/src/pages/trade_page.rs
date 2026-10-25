use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_xdex::{XDexConnection, PoolInfo, PoolPrice, TokenMetadata, USDC_MINT, NATIVE_XNT_MINT, WRAPPED_XNT_MINT};
use crate::core::rpc_base::RpcConnection;
use wasm_bindgen_futures::spawn_local;
use std::collections::HashMap;
use web_sys::window;

// Helper function to format number with thousand separators
fn format_number_with_commas(num: u64) -> String {
    let num_str = num.to_string();
    let chars: Vec<char> = num_str.chars().collect();
    let mut result = String::new();
    
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }
    
    result
}

// Helper function to format reserve amount with smart precision
fn format_reserve(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else if value < 0.000001 {
        // Very small values: use scientific notation
        format!("{:.2e}", value)
    } else if value < 0.01 {
        // Small values: show 6 decimal places
        format!("{:.6}", value)
    } else if value < 1.0 {
        // Less than 1: show 4 decimal places
        format!("{:.4}", value)
    } else if value < 1000.0 {
        // Normal values: show 2 decimal places
        format!("{:.2}", value)
    } else {
        // Large values: use thousand separators
        let integer_part = value as u64;
        let decimal_part = value - integer_part as f64;
        if decimal_part < 0.01 {
            format_number_with_commas(integer_part)
        } else {
            format!("{}.{:02}", format_number_with_commas(integer_part), (decimal_part * 100.0) as u32)
        }
    }
}

// Re-export for use in view
#[allow(dead_code)]
const USDC_MINT_ADDRESS: &str = USDC_MINT;

#[component]
pub fn TradePage(
    _session: RwSignal<Session>
) -> impl IntoView {
    // State for pools
    let (pools, set_pools) = create_signal::<Vec<PoolInfo>>(Vec::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal::<Option<String>>(None);
    
    // State for prices (pool_address -> price info)
    let (pool_prices, set_pool_prices) = create_signal::<HashMap<String, PoolPrice>>(HashMap::new());
    let (loading_prices, set_loading_prices) = create_signal(false);
    
    // State for token metadata (mint_address -> metadata)
    let (token_metadata, set_token_metadata) = create_signal::<HashMap<String, TokenMetadata>>(HashMap::new());
    
    // Fetch pools on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            log::info!("Fetching xDEX pools...");

            let xdex = XDexConnection::new();
            let target_pools = &[
                "GRYbq732zobr8fwDkqnjnaNCg5Qf5Y7vgRWBhKLFRu3j",  // XNT/MEMO
                "CAJeVEoSm1QQZccnCqYu9cnNF7TTD2fcUA3E5HQoxRvR",  // XNT/USDC.X
                "8hEhKFmb43qkcctdV94VjwQxUubZ7zCTyG7Hsb1BWcsq",  // XNT/XBLK
                "8EUkm5ChdmLm9pxKX3Q99APck1URfVqP9m9R3FQcP6Tb",
                "9oNpPyK6z1S2VCNZeAT1NfEXoLi2poMsxsycLbQdYrQe",
            ];
            match xdex.get_pools_by_addresses(target_pools).await {
                Ok(pool_list) => {
                    log::info!("Fetched {} pool(s) via getMultipleAccounts", pool_list.len());
                    
                    set_pools.set(pool_list.clone());
                    set_loading.set(false);
                    
                    // Clone pool_list for both async tasks
                    let pool_list_for_metadata = pool_list.clone();
                    let pool_list_for_prices = pool_list.clone();
                    
                    // Fetch metadata for all tokens
                    spawn_local(async move {
                        let xdex = XDexConnection::new();
                        let mut metadata_map = HashMap::new();
                        
                        // Collect all unique token mints
                        let mut token_mints = std::collections::HashSet::new();
                        for pool in pool_list_for_metadata.iter() {
                            token_mints.insert(pool.token_0_mint.clone());
                            token_mints.insert(pool.token_1_mint.clone());
                        }
                        
                        log::info!("Fetching metadata for {} unique tokens", token_mints.len());
                        
                        for mint in token_mints.iter() {
                            match xdex.get_token_metadata(mint).await {
                                Ok(metadata) => {
                                    log::info!("✓ Got metadata for {}: {:?}", mint, metadata.symbol);
                                    metadata_map.insert(mint.clone(), metadata);
                                },
                                Err(e) => {
                                    log::warn!("✗ Failed to fetch metadata for {}: {}", mint, e);
                                }
                            }
                        }
                        
                        set_token_metadata.set(metadata_map);
                        log::info!("Finished loading token metadata");
                    });
                    
                    // Fetch prices for filtered pools (in background)
                    spawn_local(async move {
                        let xdex = XDexConnection::new();
                        set_loading_prices.set(true);
                        let mut prices = HashMap::new();
                        
                        for pool in pool_list_for_prices.iter() {
                            log::info!("Fetching price for pool: {}", pool.address);
                            match xdex.get_pool_price(pool).await {
                                Ok(price_info) => {
                                    log::info!("✓ Successfully got price for pool {}: {:?}", pool.address, price_info);
                                    prices.insert(pool.address.clone(), price_info);
                                },
                                Err(e) => {
                                    log::warn!("✗ Failed to fetch price for pool {}: {}", pool.address, e);
                                }
                            }
                        }
                        
                        set_pool_prices.set(prices);
                        set_loading_prices.set(false);
                        log::info!("Finished loading prices for {} pools", pool_list_for_prices.len());
                    });
                },
                Err(e) => {
                    log::error!("Failed to fetch pools: {}", e);
                    set_error.set(Some(format!("Failed to load pools: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });
    
    // Refresh handler
    let handle_refresh = move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let xdex = XDexConnection::new();
            let target_pools = &[
                "GRYbq732zobr8fwDkqnjnaNCg5Qf5Y7vgRWBhKLFRu3j",  // XNT/MEMO
                "CAJeVEoSm1QQZccnCqYu9cnNF7TTD2fcUA3E5HQoxRvR",  // XNT/USDC.X
                "8hEhKFmb43qkcctdV94VjwQxUubZ7zCTyG7Hsb1BWcsq",  // XNT/XBLK
                "8EUkm5ChdmLm9pxKX3Q99APck1URfVqP9m9R3FQcP6Tb",
                "9oNpPyK6z1S2VCNZeAT1NfEXoLi2poMsxsycLbQdYrQe",
            ];
            match xdex.get_pools_by_addresses(target_pools).await {
                Ok(pool_list) => {
                    set_pools.set(pool_list);
                    set_loading.set(false);
                },
                Err(e) => {
                    set_error.set(Some(format!("Failed to load pools: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };
    
    // Helper to shorten address
    let shorten_address = |addr: &str| -> String {
        if addr.len() > 12 {
            format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
        } else {
            addr.to_string()
        }
    };
    
    // Helper to format pool status
    let format_status = |status: u8| -> &'static str {
        match status {
            0 => "✅ Active",
            1 => "🚫 Deposit Disabled",
            2 => "🚫 Withdraw Disabled",
            4 => "🚫 Swap Disabled",
            _ => "⚠️ Multiple Flags",
        }
    };
    
    // Tab state
    let (active_tab, set_active_tab) = create_signal("tokens".to_string());
    
    view! {
        <div class="trade-page">
            <div class="trade-page-header">
                <h2>
                    <i class="fas fa-exchange-alt"></i>
                    "Trade"
                </h2>
            </div>
            
            // Tab Navigation
            <div class="trade-tabs">
                <button 
                    class="tab-button"
                    class:active=move || active_tab.get() == "tokens"
                    on:click=move |_| set_active_tab.set("tokens".to_string())
                >
                    <i class="fas fa-coins"></i>
                    " TOKENS"
                </button>
                <button 
                    class="tab-button"
                    class:active=move || active_tab.get() == "swap"
                    on:click=move |_| set_active_tab.set("swap".to_string())
                >
                    <i class="fas fa-exchange-alt"></i>
                    " SWAP"
                </button>
                <button 
                    class="tab-button"
                    class:active=move || active_tab.get() == "autobot"
                    on:click=move |_| set_active_tab.set("autobot".to_string())
                >
                    <i class="fas fa-robot"></i>
                    " AUTO BOT"
                </button>
            </div>
            
            <div class="trade-content">
                // TOKENS Tab
                {move || {
                    if active_tab.get() == "tokens" {
                        view! {
                            <div class="tokens-tab">
                                <div class="tokens-header">
                                    <h3>"Token Pairs"</h3>
                                    <button 
                                        class="refresh-button"
                                        on:click=handle_refresh
                                        disabled=move || loading.get()
                                        title="Refresh pools"
                                    >
                                        <i class="fas fa-sync-alt" class:fa-spin=move || loading.get()></i>
                                        "Refresh"
                                    </button>
                                </div>
                                {move || {
                                    if loading.get() {
                                        view! {
                                            <div class="pools-loading">
                                                <i class="fas fa-spinner fa-spin"></i>
                                                " Loading pools from blockchain..."
                                            </div>
                                        }.into_view()
                                    } else if let Some(err) = error.get() {
                                        view! {
                                            <div class="pools-error">
                                                <i class="fas fa-exclamation-triangle"></i>
                                                " " {err}
                                            </div>
                                        }.into_view()
                                    } else if pools.get().is_empty() {
                                        view! {
                                            <div class="pools-empty">
                                                <i class="fas fa-inbox"></i>
                                                " No pools found"
                                            </div>
                                        }.into_view()
                                    } else {
                                        let pool_list = pools.get();
                                        view! {
                                            <div class="pools-container">
                                <div class="pools-table">
                                    <div class="pools-table-header">
                                        <div class="pool-col-tokens">"Token Pair"</div>
                                        <div class="pool-col-reserves">"Liquidity"</div>
                                        <div class="pool-col-price">"Price"</div>
                                    </div>
                                    <div class="pools-table-body">
                                        {
                                            // 先获取XNT的USD价格
                                            let prices_map = pool_prices.get();
                                            let xnt_usd_price = prices_map.iter()
                                                .find(|(addr, _)| **addr == "CAJeVEoSm1QQZccnCqYu9cnNF7TTD2fcUA3E5HQoxRvR")
                                                .and_then(|(_, price_info)| {
                                                    if price_info.price > 0.0 {
                                                        Some(1.0 / price_info.price)
                                                    } else {
                                                        None
                                                    }
                                                });
                                            
                                            // 计算每个池子的流动性并排序
                                            let mut pool_with_liquidity: Vec<_> = pool_list.iter().map(|pool| {
                                                let liquidity = if let Some(price_info) = prices_map.get(&pool.address) {
                                                    // 计算token0和token1的USD价值
                                                    let wxnt_mint = "So11111111111111111111111111111111111111112";
                                                    let usdc_mint = "B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq";
                                                    
                                                    let token0_usd = if pool.token_0_mint == usdc_mint {
                                                        price_info.reserve_0 * 1.0
                                                    } else if pool.token_0_mint == wxnt_mint && xnt_usd_price.is_some() {
                                                        price_info.reserve_0 * xnt_usd_price.unwrap()
                                                    } else {
                                                        0.0
                                                    };
                                                    
                                                    let token1_usd = if pool.token_1_mint == usdc_mint {
                                                        price_info.reserve_1 * 1.0
                                                    } else if pool.token_1_mint == wxnt_mint && xnt_usd_price.is_some() {
                                                        price_info.reserve_1 * xnt_usd_price.unwrap()
                                                    } else {
                                                        0.0
                                                    };
                                                    
                                                    // 如果两侧都有价值，相加；如果只有一侧，乘以2（因为CPMM池两侧价值相等）
                                                    if token0_usd > 0.0 && token1_usd > 0.0 {
                                                        token0_usd + token1_usd
                                                    } else if token0_usd > 0.0 {
                                                        token0_usd * 2.0
                                                    } else if token1_usd > 0.0 {
                                                        token1_usd * 2.0
                                                    } else {
                                                        0.0
                                                    }
                                                } else {
                                                    0.0
                                                };
                                                
                                                (pool, liquidity)
                                            }).collect();
                                            
                                            // 按流动性降序排列
                                            pool_with_liquidity.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                                            
                                            pool_with_liquidity.into_iter().map(|(pool, liquidity)| {
                                            let pool_clone = pool.clone();
                                            let address = pool.address.clone();
                                            let address_for_price = pool.address.clone();
                                            
                                            // Check if this pool should have swapped token order
                                            let should_swap = address == "WQSjCW5wPdSVcvcTEJGzEXo3VrmHx5zyRaETM42rwQC";
                                            
                                            // Swap tokens if needed
                                            let (token_0, token_1) = if should_swap {
                                                (pool.token_1_mint.clone(), pool.token_0_mint.clone())
                                            } else {
                                                (pool.token_0_mint.clone(), pool.token_1_mint.clone())
                                            };
                                            
                                            // Clone for popup usage
                                            let token_0_for_popup = token_0.clone();
                                            let token_1_for_popup = token_1.clone();
                                            
                                            // Check if token is USDC
                                            let token_0_is_usdc = token_0 == USDC_MINT;
                                            let token_1_is_usdc = token_1 == USDC_MINT;
                                            
                                            // Pre-format token display strings
                                            let token_0_display = if token_0_is_usdc { 
                                                "USDC".to_string() 
                                            } else { 
                                                shorten_address(&token_0) 
                                            };
                                            let token_1_display = if token_1_is_usdc { 
                                                "USDC".to_string() 
                                            } else { 
                                                shorten_address(&token_1) 
                                            };
                                            
                                            view! {
                                                <div class="pool-row">
                                                    <div class="pool-col-tokens pool-hover-trigger">
                                                        <div class="token-pair">
                                                            {move || {
                                                                let metadata_map = token_metadata.get();
                                                                let token_0_meta = metadata_map.get(&token_0);
                                                                
                                                                view! {
                                                                    <div class="token" class:token-usdc=token_0_is_usdc title=token_0.clone()>
                                                                        {if let Some(meta) = token_0_meta {
                                                                            if let Some(logo) = &meta.logo_uri {
                                                                                view! {
                                                                                    <img src=logo.clone() class="token-logo" alt="token logo" />
                                                                                }.into_view()
                                                                            } else {
                                                                                view! { <span></span> }.into_view()
                                                                            }
                                                                        } else {
                                                                            view! { <span></span> }.into_view()
                                                                        }}
                                                                        <span class="token-name">
                                                                            {if token_0_is_usdc {
                                                                                "USDC".to_string()
                                                                            } else if let Some(meta) = token_0_meta {
                                                                                meta.symbol.clone().unwrap_or_else(|| token_0_display.clone())
                                                                            } else {
                                                                                token_0_display.clone()
                                                                            }}
                                                                        </span>
                                                                    </div>
                                                                }
                                                            }}
                                                            <i class="fas fa-exchange-alt"></i>
                                                            {move || {
                                                                let metadata_map = token_metadata.get();
                                                                let token_1_meta = metadata_map.get(&token_1);
                                                                
                                                                view! {
                                                                    <div class="token" class:token-usdc=token_1_is_usdc title=token_1.clone()>
                                                                        {if let Some(meta) = token_1_meta {
                                                                            if let Some(logo) = &meta.logo_uri {
                                                                                view! {
                                                                                    <img src=logo.clone() class="token-logo" alt="token logo" />
                                                                                }.into_view()
                                                                            } else {
                                                                                view! { <span></span> }.into_view()
                                                                            }
                                                                        } else {
                                                                            view! { <span></span> }.into_view()
                                                                        }}
                                                                        <span class="token-name">
                                                                            {if token_1_is_usdc {
                                                                                "USDC".to_string()
                                                                            } else if let Some(meta) = token_1_meta {
                                                                                meta.symbol.clone().unwrap_or_else(|| token_1_display.clone())
                                                                            } else {
                                                                                token_1_display.clone()
                                                                            }}
                                                                        </span>
                                                                    </div>
                                                                }
                                                            }}
                                                        </div>
                                                        
                                                        // Hover popup
                                                        <div class="pool-info-popup">
                                                            <div class="popup-row">
                                                                <span class="popup-label">"Pool Id:"</span>
                                                                <span class="popup-value">{shorten_address(&address)}</span>
                                                                <button 
                                                                    class="copy-btn"
                                                                    on:click={
                                                                        let addr = address.clone();
                                                                        move |_| {
                                                                            if let Some(win) = window() {
                                                                                let _ = win.navigator().clipboard().write_text(&addr);
                                                                            }
                                                                        }
                                                                    }
                                                                >
                                                                    <i class="fas fa-copy"></i>
                                                                </button>
                                                            </div>
                                                            {move || {
                                                                let metadata_map = token_metadata.get();
                                                                let token_0_meta = metadata_map.get(&token_0_for_popup);
                                                                view! {
                                                                    <div class="popup-row">
                                                                        {if let Some(meta) = token_0_meta {
                                                                            if let Some(logo) = &meta.logo_uri {
                                                                                view! {
                                                                                    <img src=logo.clone() class="popup-token-logo" alt="token logo" />
                                                                                }.into_view()
                                                                            } else {
                                                                                view! { <span></span> }.into_view()
                                                                            }
                                                                        } else {
                                                                            view! { <span></span> }.into_view()
                                                                        }}
                                                                        <span class="popup-value">{shorten_address(&token_0_for_popup)}</span>
                                                                        <button 
                                                                            class="copy-btn"
                                                                            on:click={
                                                                                let t0 = token_0_for_popup.clone();
                                                                                move |_| {
                                                                                    if let Some(win) = window() {
                                                                                        let _ = win.navigator().clipboard().write_text(&t0);
                                                                                    }
                                                                                }
                                                                            }
                                                                        >
                                                                            <i class="fas fa-copy"></i>
                                                                        </button>
                                                                    </div>
                                                                }
                                                            }}
                                                            {move || {
                                                                let metadata_map = token_metadata.get();
                                                                let token_1_meta = metadata_map.get(&token_1_for_popup);
                                                                view! {
                                                                    <div class="popup-row">
                                                                        {if let Some(meta) = token_1_meta {
                                                                            if let Some(logo) = &meta.logo_uri {
                                                                                view! {
                                                                                    <img src=logo.clone() class="popup-token-logo" alt="token logo" />
                                                                                }.into_view()
                                                                            } else {
                                                                                view! { <span></span> }.into_view()
                                                                            }
                                                                        } else {
                                                                            view! { <span></span> }.into_view()
                                                                        }}
                                                                        <span class="popup-value">{shorten_address(&token_1_for_popup)}</span>
                                                                        <button 
                                                                            class="copy-btn"
                                                                            on:click={
                                                                                let t1 = token_1_for_popup.clone();
                                                                                move |_| {
                                                                                    if let Some(win) = window() {
                                                                                        let _ = win.navigator().clipboard().write_text(&t1);
                                                                                    }
                                                                                }
                                                                            }
                                                                        >
                                                                            <i class="fas fa-copy"></i>
                                                                        </button>
                                                                    </div>
                                                                }
                                                            }}
                                                        </div>
                                                    </div>
                                                    <div class="pool-col-reserves">
                                                        {
                                                            // 格式化流动性显示
                                                            if liquidity > 0.0 {
                                                                if liquidity >= 1_000_000.0 {
                                                                    format!("${:.2}M", liquidity / 1_000_000.0)
                                                                } else if liquidity >= 1_000.0 {
                                                                    format!("${:.2}K", liquidity / 1_000.0)
                                                                } else {
                                                                    format!("${:.2}", liquidity)
                                                                }
                                                            } else if loading_prices.get() {
                                                                "⏳".to_string()
                                                            } else {
                                                                "-".to_string()
                                                            }
                                                        }
                                                    </div>
                                                    <div class="pool-col-price">
                                                        {move || {
                                                            let prices_map = pool_prices.get();
                                                            
                                                            // 首先找到XNT/USDC.X池子获取XNT价格
                                                            let xnt_usd_price = prices_map.iter()
                                                                .find(|(addr, _)| **addr == "CAJeVEoSm1QQZccnCqYu9cnNF7TTD2fcUA3E5HQoxRvR")
                                                                .and_then(|(_, price_info)| {
                                                                    // XNT是token0, USDC.X是token1
                                                                    // price = XNT储备 / USDC储备
                                                                    // 所以 1 USDC = price XNT, 即 1 XNT = 1/price USDC
                                                                    if price_info.price > 0.0 {
                                                                        Some(1.0 / price_info.price)
                                                                    } else {
                                                                        None
                                                                    }
                                                                });
                                                            
                                                            if let Some(price_info) = prices_map.get(&address_for_price) {
                                                                // 判断当前池子是否包含XNT或USDC
                                                                let wxnt_mint = "So11111111111111111111111111111111111111112";
                                                                let usdc_mint = "B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq";
                                                                let xnt_usdc_pool = "CAJeVEoSm1QQZccnCqYu9cnNF7TTD2fcUA3E5HQoxRvR";
                                                                
                                                                let has_xnt = pool_clone.token_0_mint == wxnt_mint || pool_clone.token_1_mint == wxnt_mint;
                                                                let has_usdc = pool_clone.token_0_mint == usdc_mint || pool_clone.token_1_mint == usdc_mint;
                                                                let is_xnt_usdc_pool = address_for_price == xnt_usdc_pool;
                                                                
                                                                // 特殊处理XNT/USDC.X池子：只显示XNT的USD价格
                                                                if is_xnt_usdc_pool {
                                                                    if let Some(xnt_price) = xnt_usd_price {
                                                                        format!("${:.6}", xnt_price)
                                                                    } else {
                                                                        "N/A".to_string()
                                                                    }
                                                                } else {
                                                                    // 计算USD价格和XNT价格
                                                                    let (usd_price_opt, xnt_price_opt) = if should_swap {
                                                                        // Swap情况：显示token1的价格
                                                                        if price_info.price > 0.0 {
                                                                            let inverted_price = 1.0 / price_info.price;
                                                                            
                                                                            let usd = if has_usdc {
                                                                                // 如果有USDC，直接显示
                                                                                Some(inverted_price)
                                                                            } else if has_xnt && xnt_usd_price.is_some() {
                                                                                // 如果有XNT，通过XNT价格计算USD价格
                                                                                Some(inverted_price * xnt_usd_price.unwrap())
                                                                            } else {
                                                                                price_info.token_1_usd_price
                                                                            };
                                                                            
                                                                            let xnt = if has_xnt {
                                                                                // 如果有XNT，直接显示
                                                                                Some(inverted_price)
                                                                            } else if has_usdc && xnt_usd_price.is_some() {
                                                                                // 如果有USDC，通过XNT价格计算
                                                                                Some(inverted_price / xnt_usd_price.unwrap())
                                                                            } else if let Some(xnt_price) = xnt_usd_price {
                                                                                // 如果知道USD价格和XNT/USD价格，计算XNT价格
                                                                                usd.map(|u| u / xnt_price)
                                                                            } else {
                                                                                None
                                                                            };
                                                                            
                                                                            (usd, xnt)
                                                                        } else {
                                                                            (None, None)
                                                                        }
                                                                    } else {
                                                                        // Normal情况：显示token0的价格
                                                                        let usd = if has_usdc && pool_clone.token_0_mint == usdc_mint {
                                                                            // token0是USDC，直接用price（这是USDC的价格，即1美元）
                                                                            Some(1.0)
                                                                        } else if has_xnt && pool_clone.token_0_mint == wxnt_mint {
                                                                            // token0是XNT，需要转换
                                                                            // price = XNT储备 / other储备
                                                                            if has_usdc {
                                                                                // 另一个是USDC，所以XNT的USD价格 = 1/price
                                                                                Some(1.0 / price_info.price)
                                                                            } else {
                                                                                // 另一个不是USDC，通过XNT基准价格计算
                                                                                xnt_usd_price.map(|xnt_p| price_info.price * xnt_p)
                                                                            }
                                                                        } else if has_xnt && xnt_usd_price.is_some() {
                                                                            // token0不是XNT，但池子有XNT，通过XNT价格计算
                                                                            Some(price_info.price * xnt_usd_price.unwrap())
                                                                        } else {
                                                                            price_info.token_0_usd_price
                                                                        };
                                                                        
                                                                        let xnt = if has_xnt && pool_clone.token_0_mint == wxnt_mint {
                                                                            // token0是XNT
                                                                            Some(price_info.price)
                                                                        } else if has_usdc && pool_clone.token_0_mint == usdc_mint && xnt_usd_price.is_some() {
                                                                            // token0是USDC，通过XNT价格计算
                                                                            Some(1.0 / xnt_usd_price.unwrap())
                                                                        } else if has_usdc && xnt_usd_price.is_some() {
                                                                            // 池子有USDC（token1），通过XNT价格计算
                                                                            Some(price_info.price / xnt_usd_price.unwrap())
                                                                        } else if let Some(xnt_price) = xnt_usd_price {
                                                                            usd.map(|u| u / xnt_price)
                                                                        } else {
                                                                            None
                                                                        };
                                                                        
                                                                        (usd, xnt)
                                                                    };
                                                                    
                                                                    // 显示双价格
                                                                    let mut lines = Vec::new();
                                                                    if let Some(usd) = usd_price_opt {
                                                                        lines.push(format!("${:.6}", usd));
                                                                    }
                                                                    if let Some(xnt) = xnt_price_opt {
                                                                        lines.push(format!("{:.6} XNT", xnt));
                                                                    }
                                                                    
                                                                    if lines.is_empty() {
                                                                        format!("{:.6}", price_info.price)
                                                                    } else {
                                                                        lines.join(" | ")
                                                                    }
                                                                }
                                                            } else if loading_prices.get() {
                                                                "⏳".to_string()
                                                            } else {
                                                                "-".to_string()
                                                            }
                                                        }}
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            </div>
                        }.into_view()
                    }
                                }}
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}
                
                // SWAP Tab
                {move || {
                    if active_tab.get() == "swap" {
                        view! {
                            <div class="swap-tab">
                                <SwapForm session=_session />
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}
                
                // AUTO BOT Tab
                {move || {
                    if active_tab.get() == "autobot" {
                        view! {
                            <div class="autobot-tab">
                                <AutoBotTab session=_session />
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn SwapForm(
    session: RwSignal<Session>
) -> impl IntoView {
    // Input/Output amounts
    let (amount_in, set_amount_in) = create_signal(String::from("0.00"));
    let (amount_out, set_amount_out) = create_signal(String::from("0.00"));
    
    // Token selection
    let (token_in, set_token_in) = create_signal("XNT".to_string());
    let (token_out, set_token_out) = create_signal("MEMO".to_string());
    let (show_token_in_select, set_show_token_in_select) = create_signal(false);
    let (show_token_out_select, set_show_token_out_select) = create_signal(false);
    
    // Balance - fetch all balances directly from RPC for real-time accuracy
    let (token_balance, set_token_balance) = create_signal(0.0);
    let (token_balance_out, set_token_balance_out) = create_signal(0.0); // Output token balance
    let (balance_loading, set_balance_loading) = create_signal(false);
    let (balance_loading_out, set_balance_loading_out) = create_signal(false);
    let (balance_refresh_trigger, set_balance_refresh_trigger) = create_signal(0u32);
    
    // Token mints for balance lookup
    const MEMO_MINT: &str = "memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick";
    const USDC_X_MINT: &str = "B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq";
    
    let get_balance_for_token = move || {
        token_balance.get()
    };
    
    let get_balance_for_token_out = move || {
        token_balance_out.get()
    };
    
    // Helper function to fetch balance for any token
    let fetch_token_balance = move |token_symbol: String, is_output: bool| {
        let sess = session.get();
        
        if is_output {
            set_balance_loading_out.set(true);
        } else {
            set_balance_loading.set(true);
        }
        
        spawn_local(async move {
            match sess.get_public_key() {
                Ok(pubkey_str) => {
                    let rpc = RpcConnection::new();
                    
                    if token_symbol == "XNT" {
                        match rpc.get_balance(&pubkey_str).await {
                            Ok(balance_str) => {
                                match serde_json::from_str::<serde_json::Value>(&balance_str) {
                                    Ok(json) => {
                                        if let Some(balance_lamports) = json.get("value").and_then(|v| v.as_u64()) {
                                            let balance = balance_lamports as f64 / 1_000_000_000.0;
                                            if is_output {
                                                set_token_balance_out.set(balance);
                                            } else {
                                                set_token_balance.set(balance);
                                            }
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                            Err(_) => {}
                        }
                    } else {
                        // Fetch SPL token balance (MEMO or USDC.X)
                        let mint = if token_symbol == "MEMO" {
                            MEMO_MINT
                        } else if token_symbol == "USDC.X" {
                            USDC_X_MINT
                        } else {
                            if is_output {
                                set_token_balance_out.set(0.0);
                                set_balance_loading_out.set(false);
                            } else {
                                set_token_balance.set(0.0);
                                set_balance_loading.set(false);
                            }
                            return;
                        };
                        
                        match rpc.get_token_balance(&pubkey_str, mint).await {
                            Ok(result_str) => {
                                match serde_json::from_str::<serde_json::Value>(&result_str) {
                                    Ok(json) => {
                                        if let Some(accounts) = json.get("value").and_then(|v| v.as_array()) {
                                            if let Some(first_account) = accounts.first() {
                                                if let Some(amount) = first_account
                                                    .get("account")
                                                    .and_then(|a| a.get("data"))
                                                    .and_then(|d| d.get("parsed"))
                                                    .and_then(|p| p.get("info"))
                                                    .and_then(|i| i.get("tokenAmount"))
                                                    .and_then(|t| t.get("uiAmount"))
                                                    .and_then(|a| a.as_f64())
                                                {
                                                    if is_output {
                                                        set_token_balance_out.set(amount);
                                                    } else {
                                                        set_token_balance.set(amount);
                                                    }
                                                    if is_output {
                                                        set_balance_loading_out.set(false);
                                                    } else {
                                                        set_balance_loading.set(false);
                                                    }
                                                    return;
                                                }
                                            }
                                        }
                                        if is_output {
                                            set_token_balance_out.set(0.0);
                                        } else {
                                            set_token_balance.set(0.0);
                                        }
                                    }
                                    Err(_) => {
                                        if is_output {
                                            set_token_balance_out.set(0.0);
                                        } else {
                                            set_token_balance.set(0.0);
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                if is_output {
                                    set_token_balance_out.set(0.0);
                                } else {
                                    set_token_balance.set(0.0);
                                }
                            }
                        }
                    }
                    if is_output {
                        set_balance_loading_out.set(false);
                    } else {
                        set_balance_loading.set(false);
                    }
                }
                Err(_) => {
                    if is_output {
                        set_token_balance_out.set(0.0);
                        set_balance_loading_out.set(false);
                    } else {
                        set_token_balance.set(0.0);
                        set_balance_loading.set(false);
                    }
                }
            }
        });
    };
    
    // Fetch balance for input token when it changes or refresh is triggered
    create_effect(move |_| {
        let token_symbol = token_in.get();
        let _ = balance_refresh_trigger.get();
        fetch_token_balance(token_symbol, false);
    });
    
    // Fetch balance for output token when it changes or refresh is triggered
    create_effect(move |_| {
        let token_symbol = token_out.get();
        let _ = balance_refresh_trigger.get();
        fetch_token_balance(token_symbol, true);
    });
    
    // Slippage settings
    let (slippage, set_slippage) = create_signal(1.0);
    let (show_slippage_settings, set_show_slippage_settings) = create_signal(false);
    
    // Price impact
    let (price_impact, set_price_impact) = create_signal::<Option<f64>>(None);
    let (pool_reserves, set_pool_reserves) = create_signal::<Option<(f64, f64)>>(None);
    
    // Token prices in USD
    let (xnt_usd_price, set_xnt_usd_price) = create_signal::<Option<f64>>(None);
    let (memo_usd_price, set_memo_usd_price) = create_signal::<Option<f64>>(None);
    
    // Cache pool data to avoid repeated requests
    let (pool_cache, set_pool_cache) = create_signal::<Option<(String, f64, f64, f64)>>(None); // (pool_addr, reserve_0, reserve_1, price)
    
    // Swap state
    let (swap_status, set_swap_status) = create_signal::<Option<String>>(None);
    let (swapping, set_swapping) = create_signal(false);
    
    // Available tokens (use Rc to share between closures)
    use std::rc::Rc;
    
    #[derive(Clone)]
    struct TokenInfo {
        symbol: String,
        name: String,
        icon: String,
        mint: String,
        decimals: u8,
    }
    
    let available_tokens = Rc::new(vec![
        TokenInfo {
            symbol: "XNT".to_string(),
            name: "X1 Native Token".to_string(),
            icon: "https://app.xdex.xyz/assets/images/tokens/x1.webp".to_string(),
            mint: "So11111111111111111111111111111111111111112".to_string(),
            decimals: 9,
        },
        TokenInfo {
            symbol: "USDC.X".to_string(),
            name: "USDC.X".to_string(),
            icon: "https://x1logos.s3.us-east-1.amazonaws.com/48-usdcx.png".to_string(),
            mint: "B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq".to_string(),
            decimals: 6,
        },
        TokenInfo {
            symbol: "MEMO".to_string(),
            name: "MEMO Token".to_string(),
            icon: "https://raw.githubusercontent.com/xenartist/memo-token/refs/heads/main/metadata/memo_token-logo.png".to_string(),
            mint: "memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick".to_string(),
            decimals: 6,
        },
    ]);
    
    // Token mints and pool addresses
    const XNT_USDC_POOL: &str = "CAJeVEoSm1QQZccnCqYu9cnNF7TTD2fcUA3E5HQoxRvR";
    const XNT_MEMO_POOL: &str = "GRYbq732zobr8fwDkqnjnaNCg5Qf5Y7vgRWBhKLFRu3j";
    
    // Get token info helper
    let tokens_for_get = available_tokens.clone();
    let get_token_info = move |symbol: &str| -> TokenInfo {
        tokens_for_get.iter()
            .find(|t| t.symbol == symbol)
            .cloned()
            .unwrap_or(tokens_for_get[0].clone())
    };
    
    // For view closures
    let tokens_for_view_in = available_tokens.clone();
    let token_in_info = move || {
        let symbol = token_in.get();
        tokens_for_view_in.iter()
            .find(|t| t.symbol == symbol)
            .cloned()
            .unwrap_or(tokens_for_view_in[0].clone())
    };
    
    let tokens_for_view_out = available_tokens.clone();
    let token_out_info = move || {
        let symbol = token_out.get();
        tokens_for_view_out.iter()
            .find(|t| t.symbol == symbol)
            .cloned()
            .unwrap_or(tokens_for_view_out[0].clone())
    };
    
    // Fetch XNT USD price from XNT/USDC.X pool
    create_effect(move |_| {
        spawn_local(async move {
            let xdex = XDexConnection::new();
            if let Ok(pools) = xdex.get_pools_by_addresses(&[XNT_USDC_POOL]).await {
                if let Some(pool) = pools.first() {
                    if let Ok(price_info) = xdex.get_pool_price(pool).await {
                        let xnt_price_usd = 1.0 / price_info.price;
                        set_xnt_usd_price.set(Some(xnt_price_usd));
                    }
                }
            }
        });
    });
    
    // Fetch MEMO USD price from XNT/MEMO pool and XNT/USDC.X pool
    create_effect(move |_| {
        spawn_local(async move {
            let xdex = XDexConnection::new();
            if let Ok(pools) = xdex.get_pools_by_addresses(&[XNT_USDC_POOL, XNT_MEMO_POOL]).await {
                // First get XNT price in USD
                let xnt_price_usd = if let Some(xnt_usdc_pool) = pools.iter().find(|p| p.address == XNT_USDC_POOL) {
                    if let Ok(price_info) = xdex.get_pool_price(xnt_usdc_pool).await {
                        Some(1.0 / price_info.price)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Then calculate MEMO price in USD via XNT
                if let Some(xnt_price) = xnt_price_usd {
                    if let Some(xnt_memo_pool) = pools.iter().find(|p| p.address == XNT_MEMO_POOL) {
                        if let Ok(price_info) = xdex.get_pool_price(xnt_memo_pool).await {
                            let memo_price_in_xnt = price_info.price;
                            let memo_price_usd = memo_price_in_xnt * xnt_price;
                            set_memo_usd_price.set(Some(memo_price_usd));
                        }
                    }
                }
            }
        });
    });
    
    // Fetch pool data when token pair changes
    create_effect(move |_| {
        let token_in_val = token_in.get();
        let token_out_val = token_out.get();
        
        spawn_local(async move {
            let xdex = XDexConnection::new();
            let pool_addr = if (token_in_val == "XNT" && token_out_val == "USDC.X") || 
                               (token_in_val == "USDC.X" && token_out_val == "XNT") {
                XNT_USDC_POOL
            } else {
                XNT_MEMO_POOL
            };
            
            if let Ok(pools) = xdex.get_pools_by_addresses(&[pool_addr]).await {
                if let Some(pool) = pools.first() {
                    if let Ok(price_info) = xdex.get_pool_price(pool).await {
                        // Cache pool data
                        set_pool_cache.set(Some((
                            pool_addr.to_string(),
                            price_info.reserve_0,
                            price_info.reserve_1,
                            price_info.price,
                        )));
                    }
                }
            }
        });
    });
    
    // Calculate output and price impact when input changes (using cached pool data)
    create_effect(move |_| {
        let amount_str = amount_in.get();
        
        if let Ok(amount) = amount_str.parse::<f64>() {
            if amount > 0.0 {
                let token_in_val = token_in.get();
                let token_out_val = token_out.get();
                
                // Use cached pool data for instant calculation
                if let Some((cached_pool, reserve_0, reserve_1, _price)) = pool_cache.get() {
                    let current_pool = if (token_in_val == "XNT" && token_out_val == "USDC.X") || 
                                          (token_in_val == "USDC.X" && token_out_val == "XNT") {
                        XNT_USDC_POOL
                    } else {
                        XNT_MEMO_POOL
                    };
                    
                    // Only use cache if it's for the current pool
                    if cached_pool == current_pool {
                        // Calculate output
                        let (reserve_in, reserve_out) = if token_in_val == "XNT" {
                            (reserve_0, reserve_1)
                        } else {
                            (reserve_1, reserve_0)
                        };
                        
                        // Constant product formula: (x + Δx)(y - Δy) = xy
                        // Δy = (y * Δx) / (x + Δx)
                        let amount_out_calc = (reserve_out * amount) / (reserve_in + amount);
                        
                        // Calculate price impact
                        let market_price = reserve_out / reserve_in;
                        let execution_price = amount_out_calc / amount;
                        let impact = ((execution_price - market_price).abs() / market_price) * 100.0;
                        
                        set_amount_out.set(format!("{:.6}", amount_out_calc));
                        set_price_impact.set(Some(impact));
                        set_pool_reserves.set(Some((reserve_in, reserve_out)));
                    }
                }
            } else {
                set_amount_out.set("0.00".to_string());
                set_price_impact.set(None);
                set_pool_reserves.set(None);
            }
        } else {
            set_amount_out.set("0.00".to_string());
            set_price_impact.set(None);
            set_pool_reserves.set(None);
        }
    });
    
    // Percentage buttons handler
    let set_percentage = move |pct: f64| {
        let balance = get_balance_for_token();
        let amount = balance * pct;
        set_amount_in.set(format!("{:.4}", amount));
    };
    
    // Swap tokens logic
    let do_swap_tokens = move || {
        let temp_token = token_in.get();
        set_token_in.set(token_out.get());
        set_token_out.set(temp_token);
        
        let temp_amount = amount_in.get();
        set_amount_in.set(amount_out.get());
        set_amount_out.set(temp_amount);
    };
    
    // Swap tokens handler for button clicks
    let handle_swap_tokens = move |_ev: leptos::ev::MouseEvent| {
        do_swap_tokens();
    };
    
    // Token selection handlers
    let select_token_in = move |token: String| {
        // Prevent selecting the same token for both sides
        if token != token_out.get() {
            set_token_in.set(token);
        } else {
            // Swap if trying to select the same token
            do_swap_tokens();
        }
        set_show_token_in_select.set(false);
    };
    
    let select_token_out = move |token: String| {
        if token != token_in.get() {
            set_token_out.set(token);
        } else {
            do_swap_tokens();
        }
        set_show_token_out_select.set(false);
    };
    
    // Execute swap
    let handle_swap = move |_| {
        set_swapping.set(true);
        set_swap_status.set(Some("Processing swap...".to_string()));
        
        let amount_str = amount_in.get();
        let amount: f64 = match amount_str.parse() {
            Ok(v) => v,
            Err(_) => {
                set_swap_status.set(Some("❌ Invalid amount".to_string()));
                set_swapping.set(false);
                return;
            }
        };
        
        if amount <= 0.0 {
            set_swap_status.set(Some("❌ Amount must be greater than 0".to_string()));
            set_swapping.set(false);
            return;
        }
        
        let token_in_val = token_in.get();
        let token_out_val = token_out.get();
        let token_out_info = get_token_info(&token_out_val);
        let token_in_info = get_token_info(&token_in_val);
        
        // Clone the values for the async block
        let output_mint_owned = token_out_info.mint.clone();
        let output_decimals = token_out_info.decimals;
        
        let pool_addr = if (token_in_val == "XNT" && token_out_val == "USDC.X") || 
                           (token_in_val == "USDC.X" && token_out_val == "XNT") {
            XNT_USDC_POOL
        } else {
            XNT_MEMO_POOL
        };
        
        let amount_in_lamports = (amount * 10_f64.powi(token_in_info.decimals as i32)) as u64;
        let slippage_pct = slippage.get();
        
        spawn_local(async move {
            let result = {
                let mut sess = session.get_untracked();
                sess.wrap_and_swap(
                    pool_addr,
                    amount_in_lamports,
                    slippage_pct,
                    &output_mint_owned,
                    output_decimals,
                ).await
            };
            
            match result {
                Ok(signature) => {
                    set_swap_status.set(Some(format!("✅ Success! Tx: {}", signature)));
                    
                    // Trigger main page balance update from transaction
                    let sig_for_session = signature.clone();
                    session.update(|s| {
                        s.set_last_tx_signature(sig_for_session);
                    });
                    
                    // Trigger swap page balance refresh
                    spawn_local(async move {
                        // Small delay to let transaction finalize
                        gloo_timers::future::TimeoutFuture::new(1500).await;
                        set_balance_refresh_trigger.update(|n| *n += 1);
                    });
                },
                Err(e) => {
                    set_swap_status.set(Some(format!("❌ Failed: {}", e)));
                }
            }
            
            set_swapping.set(false);
        });
    };
    
    // Set up global click listener to close dropdowns
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    
    // This runs once when component mounts
    {
        let handle_global_click = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            // Check if click target is inside a token-select-container
            if let Some(target) = event.target() {
                if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                    // Check if the click is inside a token select area
                    if element.closest(".token-select-container").ok().flatten().is_none() {
                        // Click is outside, close all dropdowns
                        set_show_token_in_select.set(false);
                        set_show_token_out_select.set(false);
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let _ = document.add_event_listener_with_callback(
                    "click",
                    handle_global_click.as_ref().unchecked_ref()
                );
            }
        }
        
        handle_global_click.forget();
    }
    
    view! {
        <div class="swap-form">
            // Header with settings
            <div class="swap-header">
                <h3 class="swap-title">"Swap"</h3>
                <button 
                    class="settings-btn"
                    on:click=move |_| set_show_slippage_settings.set(!show_slippage_settings.get())
                >
                    <i class="fas fa-cog"></i>
                </button>
            </div>
            
            // Slippage Settings
            {move || {
                if show_slippage_settings.get() {
                    view! {
                        <div class="slippage-settings">
                            <div class="slippage-label">"Slippage Tolerance"</div>
                            <div class="slippage-options">
                                <button 
                                    class="slippage-btn"
                                    class:active=move || slippage.get() == 0.5
                                    on:click=move |_| set_slippage.set(0.5)
                                >
                                    "0.5%"
                                </button>
                                <button 
                                    class="slippage-btn"
                                    class:active=move || slippage.get() == 1.0
                                    on:click=move |_| set_slippage.set(1.0)
                                >
                                    "1.0%"
                                </button>
                                <button 
                                    class="slippage-btn"
                                    class:active=move || slippage.get() == 2.0
                                    on:click=move |_| set_slippage.set(2.0)
                                >
                                    "2.0%"
                                </button>
                                <input 
                                    type="number"
                                    class="slippage-custom-input"
                                    placeholder="Custom"
                                    step="0.1"
                                    min="0.1"
                                    max="50"
                                    prop:value=move || slippage.get().to_string()
                                    on:input=move |ev| {
                                        if let Ok(val) = event_target_value(&ev).parse::<f64>() {
                                            if val >= 0.1 && val <= 50.0 {
                                                set_slippage.set(val);
                                            }
                                        }
                                    }
                                />
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}
            
            // Input Section
            
            // Input Section
            <div class="swap-card">
                <div class="swap-card-header">
                    <span class="swap-label">"You Pay"</span>
                    <span class="swap-balance">
                        "Balance: " {move || format!("{:.4}", get_balance_for_token())}
                    </span>
                </div>
                <div class="swap-input-row">
                    <input 
                        type="text"
                        inputmode="decimal"
                        class="swap-amount-input"
                        placeholder="0.00"
                        prop:value=move || amount_in.get()
                        on:input=move |ev| {
                            set_amount_in.set(event_target_value(&ev));
                        }
                    />
                    <div class="token-select-container">
                        <button 
                            class="token-select-btn"
                            on:click=move |_| set_show_token_in_select.set(!show_token_in_select.get())
                        >
                            {
                                let tokens_for_btn_in = available_tokens.clone();
                                move || {
                                    let symbol = token_in.get();
                                    let token = tokens_for_btn_in.iter().find(|t| t.symbol == symbol);
                                    if let Some(t) = token {
                                        view! {
                                            <>
                                                <img src=t.icon.clone() class="token-icon" />
                                                <span>{t.symbol.clone()}</span>
                                            </>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <>
                                                <span>"Select Token"</span>
                                            </>
                                        }.into_view()
                                    }
                                }
                            }
                            <i class="fas fa-chevron-down"></i>
                        </button>
                        
                        // Token dropdown
                        {
                            let tokens_for_dropdown_in = available_tokens.clone();
                            move || {
                                if show_token_in_select.get() {
                                    view! {
                                        <div class="token-dropdown">
                                            {tokens_for_dropdown_in.iter().map(|token| {
                                                let token_clone = token.clone();
                                                view! {
                                                    <button 
                                                        class="token-dropdown-item"
                                                        on:click=move |_| select_token_in(token_clone.symbol.clone())
                                                    >
                                                        <img src=token.icon.clone() class="token-dropdown-icon" />
                                                        <div class="token-dropdown-info">
                                                            <div class="token-dropdown-symbol">{token.symbol.clone()}</div>
                                                            <div class="token-dropdown-name">{token.name.clone()}</div>
                                                        </div>
                                                    </button>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }
                            }
                        }
                    </div>
                </div>
                <div class="swap-usd-value">
                    "≈ $" {move || {
                        if let Ok(amount) = amount_in.get().parse::<f64>() {
                            let token_symbol = token_in.get();
                            if token_symbol == "XNT" {
                                if let Some(xnt_price) = xnt_usd_price.get() {
                                    format!("{:.2}", amount * xnt_price)
                                } else {
                                    "...".to_string()
                                }
                            } else if token_symbol == "USDC.X" {
                                format!("{:.2}", amount * 1.0)
                            } else if token_symbol == "MEMO" {
                                if let Some(memo_price) = memo_usd_price.get() {
                                    format!("{:.2}", amount * memo_price)
                                } else {
                                    "...".to_string()
                                }
                            } else {
                                "...".to_string()
                            }
                        } else {
                            "0.00".to_string()
                        }
                    }} " USD"
                </div>
                
                // Percentage buttons
                <div class="percentage-buttons">
                    <button class="pct-btn" on:click=move |_| set_percentage(0.25)>"25%"</button>
                    <button class="pct-btn" on:click=move |_| set_percentage(0.50)>"50%"</button>
                    <button class="pct-btn" on:click=move |_| set_percentage(0.75)>"75%"</button>
                    <button class="pct-btn" on:click=move |_| set_percentage(1.0)>"MAX"</button>
                </div>
            </div>
            
            // Swap Direction Button
            <div class="swap-arrow-container">
                <button class="swap-arrow-btn" on:click=handle_swap_tokens>
                    <i class="fas fa-arrow-down"></i>
                </button>
            </div>
            
            // Output Section
            <div class="swap-card">
                <div class="swap-card-header">
                    <span class="swap-label">"You Receive"</span>
                    <span class="swap-balance">
                        "Balance: "
                        {move || {
                            if balance_loading_out.get() {
                                "Loading...".to_string()
                            } else {
                                format!("{:.4}", get_balance_for_token_out())
                            }
                        }}
                    </span>
                </div>
                <div class="swap-input-row">
                    <input 
                        type="text"
                        class="swap-amount-input"
                        placeholder="0.00"
                        prop:value=move || amount_out.get()
                        disabled=true
                    />
                    <div class="token-select-container">
                        <button 
                            class="token-select-btn"
                            on:click=move |_| set_show_token_out_select.set(!show_token_out_select.get())
                        >
                            {
                                let tokens_for_btn_out = available_tokens.clone();
                                move || {
                                    let symbol = token_out.get();
                                    let token = tokens_for_btn_out.iter().find(|t| t.symbol == symbol);
                                    if let Some(t) = token {
                                        view! {
                                            <>
                                                <img src=t.icon.clone() class="token-icon" />
                                                <span>{t.symbol.clone()}</span>
                                            </>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <>
                                                <span>"Select Token"</span>
                                            </>
                                        }.into_view()
                                    }
                                }
                            }
                            <i class="fas fa-chevron-down"></i>
                        </button>
                        
                        // Token dropdown
                        {
                            let tokens_for_dropdown_out = available_tokens.clone();
                            move || {
                                if show_token_out_select.get() {
                                    view! {
                                        <div class="token-dropdown">
                                            {tokens_for_dropdown_out.iter().map(|token| {
                                                let token_clone = token.clone();
                                                view! {
                                                    <button 
                                                        class="token-dropdown-item"
                                                        on:click=move |_| select_token_out(token_clone.symbol.clone())
                                                    >
                                                        <img src=token.icon.clone() class="token-dropdown-icon" />
                                                        <div class="token-dropdown-info">
                                                            <div class="token-dropdown-symbol">{token.symbol.clone()}</div>
                                                            <div class="token-dropdown-name">{token.name.clone()}</div>
                                                        </div>
                                                    </button>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }
                            }
                        }
                    </div>
                </div>
                <div class="swap-usd-value">
                    "≈ $" {move || {
                        if let Ok(amount) = amount_out.get().parse::<f64>() {
                            let token_symbol = token_out.get();
                            if token_symbol == "XNT" {
                                if let Some(xnt_price) = xnt_usd_price.get() {
                                    format!("{:.2}", amount * xnt_price)
                                } else {
                                    "...".to_string()
                                }
                            } else if token_symbol == "USDC.X" {
                                format!("{:.2}", amount * 1.0)
                            } else if token_symbol == "MEMO" {
                                if let Some(memo_price) = memo_usd_price.get() {
                                    format!("{:.2}", amount * memo_price)
                                } else {
                                    "...".to_string()
                                }
                            } else {
                                "...".to_string()
                            }
                        } else {
                            "0.00".to_string()
                        }
                    }} " USD"
                </div>
            </div>
            
            // Price Impact Warning
            {move || {
                if let Some(impact) = price_impact.get() {
                    let warning_class = if impact > 5.0 {
                        "price-impact-high"
                    } else if impact > 2.0 {
                        "price-impact-medium"
                    } else {
                        "price-impact-low"
                    };
                    
                    view! {
                        <div class=format!("price-impact {}", warning_class)>
                            <div class="price-impact-label">
                                <i class="fas fa-info-circle"></i>
                                " Price Impact"
                            </div>
                            <div class="price-impact-value">
                                {format!("{:.2}%", impact)}
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}
            
            // Swap Button
            <button 
                class="swap-execute-btn"
                on:click=handle_swap
                disabled=move || swapping.get()
            >
                {move || if swapping.get() {
                    "⏳ Swapping..."
                } else {
                    "Swap"
                }}
            </button>
            
            // Status Message
            {move || swap_status.get().map(|status| {
                view! {
                    <div class="swap-status">
                        {status}
                    </div>
                }
            })}
        </div>
    }
}

// ==================== AUTO BOT COMPONENTS ====================

/// Bot type enum
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BotType {
    DCA,        // Dollar Cost Averaging
}

impl BotType {
    fn as_str(&self) -> &'static str {
        match self {
            BotType::DCA => "DCA",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            BotType::DCA => "Dollar Cost Averaging - Buy at regular intervals",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            BotType::DCA => "fa-chart-line",
        }
    }
}

/// Bot status enum
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BotStatus {
    Running,
    Paused,
    Completed,
    Error,
}

impl BotStatus {
    fn as_str(&self) -> &'static str {
        match self {
            BotStatus::Running => "Running",
            BotStatus::Paused => "Paused",
            BotStatus::Completed => "Completed",
            BotStatus::Error => "Error",
        }
    }
    
    fn icon(&self) -> &'static str {
        match self {
            BotStatus::Running => "fa-play-circle",
            BotStatus::Paused => "fa-pause-circle",
            BotStatus::Completed => "fa-check-circle",
            BotStatus::Error => "fa-exclamation-circle",
        }
    }
    
    fn class(&self) -> &'static str {
        match self {
            BotStatus::Running => "status-running",
            BotStatus::Paused => "status-paused",
            BotStatus::Completed => "status-completed",
            BotStatus::Error => "status-error",
        }
    }
}

/// DCA frequency enum
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DCAFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

impl DCAFrequency {
    fn as_str(&self) -> &'static str {
        match self {
            DCAFrequency::Hourly => "Hourly",
            DCAFrequency::Daily => "Daily",
            DCAFrequency::Weekly => "Weekly",
            DCAFrequency::Monthly => "Monthly",
        }
    }
}

/// Bot configuration struct
#[derive(Clone, Debug)]
pub struct BotConfig {
    pub id: String,
    pub name: String,
    pub bot_type: BotType,
    pub token_in: String,
    pub token_out: String,
    pub status: BotStatus,
    // DCA specific
    pub dca_amount: Option<f64>,
    pub dca_frequency: Option<DCAFrequency>,
    pub dca_total_investment: Option<f64>,
    pub dca_invested: Option<f64>,
    // Stats
    pub executed_count: u32,
    pub total_pnl: f64,
    pub created_at: String,
}

/// Main Auto Bot Tab Component
#[component]
fn AutoBotTab(
    session: RwSignal<Session>
) -> impl IntoView {
    // Bot list state
    let (bots, set_bots) = create_signal::<Vec<BotConfig>>(Vec::new());
    let (show_create_dialog, set_show_create_dialog) = create_signal(false);
    // For future use: editing existing bots
    let (_editing_bot, _set_editing_bot) = create_signal::<Option<BotConfig>>(None);
    
    // Demo: Add some sample bots for UI testing (in production, load from storage)
    create_effect(move |_| {
        // Load bots from local storage or initialize empty
        // For now, we start with an empty list
        set_bots.set(Vec::new());
    });
    
    // Handle create new bot
    let handle_create_bot = move |config: BotConfig| {
        set_bots.update(|list| {
            list.push(config);
        });
        set_show_create_dialog.set(false);
    };
    
    // Handle delete bot
    let handle_delete_bot = move |bot_id: String| {
        set_bots.update(|list| {
            list.retain(|b| b.id != bot_id);
        });
    };
    
    // Handle toggle bot status
    let handle_toggle_bot = move |bot_id: String| {
        set_bots.update(|list| {
            if let Some(bot) = list.iter_mut().find(|b| b.id == bot_id) {
                bot.status = match bot.status {
                    BotStatus::Running => BotStatus::Paused,
                    BotStatus::Paused => BotStatus::Running,
                    _ => bot.status,
                };
            }
        });
    };
    
    view! {
        <div class="autobot-content">
            // Header with create button
            <div class="autobot-header">
                <h3>"Trading Bots"</h3>
                <button 
                    class="create-bot-btn"
                    on:click=move |_| set_show_create_dialog.set(true)
                >
                    <i class="fas fa-plus"></i>
                    " Create Bot"
                </button>
            </div>
            
            // Bot cards or empty state
            {move || {
                let bot_list = bots.get();
                if bot_list.is_empty() {
                    view! {
                        <div class="bots-empty">
                            <i class="fas fa-robot"></i>
                            <h4>"No Trading Bots Yet"</h4>
                            <p>"Create your first automated trading bot to get started."</p>
                            <button 
                                class="create-first-bot-btn"
                                on:click=move |_| set_show_create_dialog.set(true)
                            >
                                <i class="fas fa-plus"></i>
                                " Create Your First Bot"
                            </button>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="bots-grid">
                            {bot_list.into_iter().map(|bot| {
                                let bot_id = bot.id.clone();
                                let bot_id_for_toggle = bot.id.clone();
                                let bot_id_for_delete = bot.id.clone();
                                view! {
                                    <BotCard 
                                        bot=bot
                                        on_toggle=move |_| handle_toggle_bot(bot_id_for_toggle.clone())
                                        on_delete=move |_| handle_delete_bot(bot_id_for_delete.clone())
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_view()
                }
            }}
            
            // Create Bot Dialog
            {move || {
                if show_create_dialog.get() {
                    view! {
                        <CreateBotDialog 
                            on_close=move |_| set_show_create_dialog.set(false)
                            on_create=handle_create_bot
                        />
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}
        </div>
    }
}

/// Bot Card Component
#[component]
fn BotCard(
    bot: BotConfig,
    on_toggle: impl Fn(()) + 'static,
    on_delete: impl Fn(()) + 'static,
) -> impl IntoView {
    let status_class = bot.status.class();
    let can_toggle = bot.status == BotStatus::Running || bot.status == BotStatus::Paused;
    
    view! {
        <div class="bot-card">
            <div class="bot-card-header">
                <div class="bot-type-badge">
                    <i class=format!("fas {}", bot.bot_type.icon())></i>
                    {bot.bot_type.as_str()}
                </div>
                <div class=format!("bot-status {}", status_class)>
                    <i class=format!("fas {}", bot.status.icon())></i>
                    {bot.status.as_str()}
                </div>
            </div>
            
            <div class="bot-card-body">
                <div class="bot-name">{bot.name.clone()}</div>
                <div class="bot-pair">
                    <span class="token-from">{bot.token_in.clone()}</span>
                    <i class="fas fa-arrow-right"></i>
                    <span class="token-to">{bot.token_out.clone()}</span>
                </div>
                
                // Bot-specific info
                {match bot.bot_type {
                    BotType::DCA => {
                        view! {
                            <div class="bot-details">
                                <div class="detail-row">
                                    <span class="detail-label">"Amount:"</span>
                                    <span class="detail-value">{format!("{:.2}", bot.dca_amount.unwrap_or(0.0))} " " {bot.token_in.clone()}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-label">"Frequency:"</span>
                                    <span class="detail-value">{bot.dca_frequency.map(|f| f.as_str()).unwrap_or("N/A")}</span>
                                </div>
                            </div>
                        }.into_view()
                    },
                }}
                
                // Stats
                <div class="bot-stats">
                    <div class="stat">
                        <span class="stat-label">"Executed"</span>
                        <span class="stat-value">{bot.executed_count}</span>
                    </div>
                    <div class="stat">
                        <span class="stat-label">"P&L"</span>
                        <span class=format!("stat-value {}", if bot.total_pnl >= 0.0 { "success" } else { "danger" })>
                            {if bot.total_pnl >= 0.0 { "+" } else { "" }}
                            {format!("{:.2}%", bot.total_pnl)}
                        </span>
                    </div>
                </div>
            </div>
            
            <div class="bot-card-actions">
                {if can_toggle {
                    let toggle_text = if bot.status == BotStatus::Running { "Pause" } else { "Start" };
                    let toggle_icon = if bot.status == BotStatus::Running { "fa-pause" } else { "fa-play" };
                    view! {
                        <button class="bot-action-btn toggle-btn" on:click=move |_| on_toggle(())>
                            <i class=format!("fas {}", toggle_icon)></i>
                            {toggle_text}
                        </button>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }}
                <button class="bot-action-btn delete-btn" on:click=move |_| on_delete(())>
                    <i class="fas fa-trash"></i>
                    "Delete"
                </button>
            </div>
        </div>
    }
}

/// Create Bot Dialog Component
#[component]
fn CreateBotDialog(
    on_close: impl Fn(()) + 'static + Clone,
    on_create: impl Fn(BotConfig) + 'static,
) -> impl IntoView {
    // Dialog state
    let (selected_type, set_selected_type) = create_signal(BotType::DCA);
    let (bot_name, set_bot_name) = create_signal(String::new());
    let (token_in, set_token_in) = create_signal("XNT".to_string());
    let (token_out, set_token_out) = create_signal("MEMO".to_string());
    
    // DCA config
    let (dca_amount, set_dca_amount) = create_signal("10".to_string());
    let (dca_frequency, set_dca_frequency) = create_signal(DCAFrequency::Daily);
    let (dca_total, set_dca_total) = create_signal("1000".to_string());
    
    // Slippage (shared)
    let (slippage, set_slippage) = create_signal("1.0".to_string());
    
    // Validation
    let (error_msg, set_error_msg) = create_signal::<Option<String>>(None);
    
    // Clone on_close for different closures
    let on_close_for_overlay = on_close.clone();
    let on_close_for_header = on_close.clone();
    let on_close_for_footer = on_close.clone();
    
    // Handle form submit
    let handle_submit = move |_| {
        // Validate and create bot config
        let name = if bot_name.get().is_empty() {
            format!("{} Bot #{}", selected_type.get().as_str(), js_sys::Date::now() as u64 % 10000)
        } else {
            bot_name.get()
        };
        
        let mut config = BotConfig {
            id: format!("bot_{}", js_sys::Date::now() as u64),
            name,
            bot_type: selected_type.get(),
            token_in: token_in.get(),
            token_out: token_out.get(),
            status: BotStatus::Paused,
            dca_amount: None,
            dca_frequency: None,
            dca_total_investment: None,
            dca_invested: None,
            executed_count: 0,
            total_pnl: 0.0,
            created_at: js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default(),
        };
        
        // Fill type-specific config
        match selected_type.get() {
            BotType::DCA => {
                config.dca_amount = dca_amount.get().parse().ok();
                config.dca_frequency = Some(dca_frequency.get());
                config.dca_total_investment = dca_total.get().parse().ok();
                config.dca_invested = Some(0.0);
            },
        }
        
        on_create(config);
    };
    
    view! {
        <div class="dialog-overlay" on:click=move |_| on_close_for_overlay(())>
            <div class="create-bot-dialog" on:click=|e| e.stop_propagation()>
                <div class="dialog-header">
                    <h3>"Create Trading Bot"</h3>
                    <button class="dialog-close-btn" on:click=move |_| on_close_for_header(())>
                        <i class="fas fa-times"></i>
                    </button>
                </div>
                
                <div class="dialog-body">
                    // Bot Type Selector (Shared Component)
                    <div class="config-section">
                        <label class="config-label">"Bot Type"</label>
                        <div class="bot-type-selector">
                            <button 
                                class="bot-type-btn"
                                class:active=move || selected_type.get() == BotType::DCA
                                on:click=move |_| set_selected_type.set(BotType::DCA)
                            >
                                <i class="fas fa-chart-line"></i>
                                <span>"DCA"</span>
                                <small>"Buy at intervals"</small>
                            </button>
                        </div>
                    </div>
                    
                    // Bot Name (Shared Component)
                    <div class="config-section">
                        <label class="config-label">"Bot Name (Optional)"</label>
                        <input 
                            type="text"
                            class="config-input"
                            placeholder="My Trading Bot"
                            prop:value=move || bot_name.get()
                            on:input=move |ev| set_bot_name.set(event_target_value(&ev))
                        />
                    </div>
                    
                    // Token Pair Selector (Shared Component)
                    <div class="config-section">
                        <label class="config-label">"Trading Pair"</label>
                        <div class="token-pair-selector">
                            <select 
                                class="token-select"
                                prop:value=move || token_in.get()
                                on:change=move |ev| set_token_in.set(event_target_value(&ev))
                            >
                                <option value="XNT">"XNT"</option>
                                <option value="MEMO">"MEMO"</option>
                                <option value="USDC.X">"USDC.X"</option>
                            </select>
                            <span class="pair-arrow">
                                <i class="fas fa-arrow-right"></i>
                            </span>
                            <select 
                                class="token-select"
                                prop:value=move || token_out.get()
                                on:change=move |ev| set_token_out.set(event_target_value(&ev))
                            >
                                <option value="MEMO">"MEMO"</option>
                                <option value="XNT">"XNT"</option>
                                <option value="USDC.X">"USDC.X"</option>
                            </select>
                        </div>
                    </div>
                    
                    // Type-specific config panels
                    {move || {
                        match selected_type.get() {
                            BotType::DCA => {
                                view! {
                                    <DCAConfigPanel 
                                        amount=dca_amount
                                        set_amount=set_dca_amount
                                        frequency=dca_frequency
                                        set_frequency=set_dca_frequency
                                        total=dca_total
                                        set_total=set_dca_total
                                        token_in=token_in.get()
                                    />
                                }.into_view()
                            },
                        }
                    }}
                    
                    // Advanced Settings (Shared Component)
                    <div class="config-section advanced-section">
                        <div class="advanced-header">
                            <label class="config-label">"Advanced Settings"</label>
                        </div>
                        <div class="advanced-content">
                            <div class="config-row">
                                <label>"Slippage Tolerance"</label>
                                <div class="slippage-input-group">
                                    <input 
                                        type="number"
                                        class="config-input small"
                                        step="0.1"
                                        min="0.1"
                                        max="10"
                                        prop:value=move || slippage.get()
                                        on:input=move |ev| set_slippage.set(event_target_value(&ev))
                                    />
                                    <span class="input-suffix">"%"</span>
                                </div>
                            </div>
                        </div>
                    </div>
                    
                    // Error message
                    {move || error_msg.get().map(|msg| {
                        view! {
                            <div class="error-message">
                                <i class="fas fa-exclamation-triangle"></i>
                                {msg}
                            </div>
                        }
                    })}
                </div>
                
                <div class="dialog-footer">
                    <button class="dialog-btn cancel-btn" on:click=move |_| on_close_for_footer(())>
                        "Cancel"
                    </button>
                    <button class="dialog-btn create-btn" on:click=handle_submit>
                        <i class="fas fa-plus"></i>
                        " Create Bot"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// DCA Config Panel Component
#[component]
fn DCAConfigPanel(
    amount: ReadSignal<String>,
    set_amount: WriteSignal<String>,
    frequency: ReadSignal<DCAFrequency>,
    set_frequency: WriteSignal<DCAFrequency>,
    total: ReadSignal<String>,
    set_total: WriteSignal<String>,
    token_in: String,
) -> impl IntoView {
    view! {
        <div class="type-config-panel dca-config">
            <div class="config-section">
                <label class="config-label">"Amount per Order"</label>
                <div class="input-with-suffix">
                    <input 
                        type="number"
                        class="config-input"
                        step="0.01"
                        min="0"
                        placeholder="10"
                        prop:value=move || amount.get()
                        on:input=move |ev| set_amount.set(event_target_value(&ev))
                    />
                    <span class="input-suffix">{token_in.clone()}</span>
                </div>
            </div>
            
            <div class="config-section">
                <label class="config-label">"Frequency"</label>
                <div class="frequency-selector">
                    <button 
                        class="freq-btn"
                        class:active=move || frequency.get() == DCAFrequency::Hourly
                        on:click=move |_| set_frequency.set(DCAFrequency::Hourly)
                    >
                        "Hourly"
                    </button>
                    <button 
                        class="freq-btn"
                        class:active=move || frequency.get() == DCAFrequency::Daily
                        on:click=move |_| set_frequency.set(DCAFrequency::Daily)
                    >
                        "Daily"
                    </button>
                    <button 
                        class="freq-btn"
                        class:active=move || frequency.get() == DCAFrequency::Weekly
                        on:click=move |_| set_frequency.set(DCAFrequency::Weekly)
                    >
                        "Weekly"
                    </button>
                    <button 
                        class="freq-btn"
                        class:active=move || frequency.get() == DCAFrequency::Monthly
                        on:click=move |_| set_frequency.set(DCAFrequency::Monthly)
                    >
                        "Monthly"
                    </button>
                </div>
            </div>
            
            <div class="config-section">
                <label class="config-label">"Total Investment Budget"</label>
                <div class="input-with-suffix">
                    <input 
                        type="number"
                        class="config-input"
                        step="1"
                        min="0"
                        placeholder="1000"
                        prop:value=move || total.get()
                        on:input=move |ev| set_total.set(event_target_value(&ev))
                    />
                    <span class="input-suffix">{token_in}</span>
                </div>
                <div class="config-hint">
                    "Bot will stop when total investment is reached"
                </div>
            </div>
        </div>
    }
}

