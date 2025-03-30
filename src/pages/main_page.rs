use leptos::*;
use crate::core::rpc::RpcConnection;

#[component]
pub fn MainPage() -> impl IntoView {
    let (version_status, set_version_status) = create_signal(String::from("Testing RPC connection..."));
    let (blockhash_status, set_blockhash_status) = create_signal(String::from("Getting latest blockhash..."));
    
    // use a sample pubkey, we will get it from wallet state later
    let pubkey = "7C4jsPZpht42Tw6MjXWF56Q5RQUocjBCAQS3DgXyLhyB";
    
    // test rpc connection
    spawn_local(async move {
        let rpc = RpcConnection::new();
        
        // test getVersion
        match rpc.get_version().await {
            Ok(version) => {
                set_version_status.set(format!("✅ RPC Version: {}", version));
            }
            Err(e) => {
                set_version_status.set(format!("❌ RPC Version Error: {}", e));
            }
        }

        // test getLatestBlockhash
        match rpc.get_latest_blockhash().await {
            Ok(blockhash) => {
                set_blockhash_status.set(format!("✅ Latest Blockhash: {}", blockhash));
            }
            Err(e) => {
                set_blockhash_status.set(format!("❌ Blockhash Error: {}", e));
            }
        }
    });

    view! {
        <div class="main-page">
            // top bar
            <div class="top-bar">
                <div class="wallet-address">
                    <span class="address-label">"Wallet: "</span>
                    <span class="address-value" title={pubkey}>
                        {format!("{}...{}", &pubkey[..4], &pubkey[pubkey.len()-4..])}
                    </span>
                </div>
            </div>

            // rpc status
            <div class="rpc-status">
                <h3>"X1 RPC Status"</h3>
                <p>{version_status}</p>
                <p>{blockhash_status}</p>
            </div>
        </div>
    }
} 