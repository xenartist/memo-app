use leptos::*;
use crate::core::rpc::RpcConnection;

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
            <div>
                <h3>"Latest Burn Shard Data:"</h3>
                <pre>{burn_shard_data}</pre>
            </div>
        </div>
    }
} 