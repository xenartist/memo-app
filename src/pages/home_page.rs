use leptos::*;
use crate::core::rpc::RpcConnection;
use crate::pages::memo_card::MemoCard;

#[component]
pub fn HomePage() -> impl IntoView {
    let (burn_shard_data, set_burn_shard_data) = create_signal(String::new());
    
    // Create RPC connection and fetch latest burn shard data
    spawn_local(async move {
        let rpc = RpcConnection::new();
        match rpc.get_latest_burn_shard().await {
            Ok(result) => {
                set_burn_shard_data.set(result);
            },
            Err(e) => {
                set_burn_shard_data.set(format!("Error: {}", e));
            }
        }
    });

    view! {
        <div class="home-page">
            <h2>"Home"</h2>
            
            <div class="memo-cards">
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="5KN1ng2dSqZ3LBPgqyJVgBxnxwwBWAzm6wH7GNvQyAL4m5EUrNUCMz2hpC1w8mxDp1rof7rHyqX1KyqtZULmPmw".to_string()
                    pubkey="DuRBUwWoqMHwHiZVvQwz5FdZA4fKYxDBxqicdDVxpEZx".to_string()
                    blocktime=1709668246
                    amount=1.5
                />
            </div>

            <div class="burn-shard-section">
                <h3>"Latest Burn Shard Data:"</h3>
                <pre>{burn_shard_data}</pre>
            </div>
        </div>
    }
} 