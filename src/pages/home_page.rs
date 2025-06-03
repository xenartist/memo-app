use leptos::*;
use crate::core::rpc_base::RpcConnection;
use crate::pages::memo_card::MemoCard;
use base64;
use solana_sdk::pubkey::Pubkey;

// Burn Record
#[derive(Clone, Debug)]
struct BurnRecord {
    pubkey: Pubkey,      // 32 bytes
    signature: String,    // 88 bytes (base58 encoded signature)
    slot: u64,           // 8 bytes
    blocktime: i64,      // 8 bytes
    amount: u64,         // 8 bytes - token burn amount
}

#[component]
pub fn HomePage() -> impl IntoView {
    let (burn_records, set_burn_records) = create_signal(Vec::new());
    let (is_loading, set_is_loading) = create_signal(true);
    let (error_message, set_error_message) = create_signal(String::new());
    
    // Fetch latest burn shard data
    spawn_local(async move {
        set_is_loading.set(true);
        set_error_message.set(String::new());
        
        let rpc = RpcConnection::new();
        match rpc.get_latest_burn_shard().await {
            Ok(account_info_str) => {
                // Parse JSON response
                if let Ok(account_info) = serde_json::from_str::<serde_json::Value>(&account_info_str) {
                    // Get base64 encoded data
                    if let Some(data) = account_info["value"]["data"].get(0).and_then(|v| v.as_str()) {
                        // Decode base64 data
                        if let Ok(decoded) = base64::decode(data) {
                            let mut records = Vec::new();
                            let mut data = &decoded[8..]; // Skip discriminator

                            // Read current_index (1 byte)
                            let _current_index = data[0];
                            data = &data[1..];
                            
                            // Read record vector length
                            let vec_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
                            data = &data[4..];

                            // Parse each record
                            for _ in 0..vec_len {
                                // Parse pubkey (32 bytes)
                                let mut pubkey_bytes = [0u8; 32];
                                pubkey_bytes.copy_from_slice(&data[..32]);
                                let record_pubkey = Pubkey::new_from_array(pubkey_bytes);
                                data = &data[32..];
                                
                                // Parse signature
                                let sig_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
                                data = &data[4..];
                                let signature = String::from_utf8(data[..sig_len].to_vec())
                                    .unwrap_or_default();
                                data = &data[sig_len..];
                                
                                // Parse slot (8 bytes)
                                let slot = u64::from_le_bytes(data[..8].try_into().unwrap());
                                data = &data[8..];
                                
                                // Parse blocktime (8 bytes)
                                let blocktime = i64::from_le_bytes(data[..8].try_into().unwrap());
                                data = &data[8..];
                                
                                // Parse amount (8 bytes)
                                let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
                                data = &data[8..];

                                records.push(BurnRecord {
                                    pubkey: record_pubkey,
                                    signature,
                                    slot,
                                    blocktime,
                                    amount,
                                });
                            }

                            // Sort by amount in descending order
                            records.sort_by(|a, b| b.amount.cmp(&a.amount));
                            set_burn_records.set(records);
                        }
                    }
                }
            },
            Err(e) => {
                log::error!("Failed to fetch burn shard data: {}", e);
                set_error_message.set(format!("Failed to fetch burn shard data: {}", e));
            }
        }
        
        // 在最后设置 loading 为 false
        set_is_loading.set(false);
    });

    view! {
        <div class="home-page">
            <h2>"Home"</h2>
            
            <div class="memo-cards">
                {move || {
                    if is_loading.get() {
                        view! {
                            <div class="loading-container">
                                <div class="loading-spinner"></div>
                                <p class="loading-text">"Loading burn records..."</p>
                            </div>
                        }.into_view()
                    } else if !error_message.get().is_empty() {
                        view! {
                            <div class="error-container">
                                <p class="error-message">{error_message.get()}</p>
                                <button class="retry-button" on:click=move |_| {
                                    // 重试逻辑
                                    window().location().reload().unwrap();
                                }>"Retry"</button>
                            </div>
                        }.into_view()
                    } else if burn_records.get().is_empty() {
                        view! {
                            <div class="empty-state">
                                <p class="empty-message">"No burn records found"</p>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <For
                                each=move || burn_records.get()
                                key=|record| record.signature.clone()
                                children=move |record: BurnRecord| {
                                    view! {
                                        <MemoCard
                                            image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                                            signature=record.signature
                                            pubkey=record.pubkey.to_string()
                                            blocktime=record.blocktime
                                            amount={(record.amount as f64) / 1_000_000_000.0}
                                        />
                                    }
                                }
                            />
                        }.into_view()
                    }
                }}
            </div>
        </div>
    }
} 