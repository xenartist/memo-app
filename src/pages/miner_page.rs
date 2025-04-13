use leptos::*;
use crate::core::session::Session;

#[component]
pub fn MinerPage(
    session: RwSignal<Session>
) -> impl IntoView {
    let wallet_address = move || {
        match session.get().get_public_key() {
            Ok(addr) => addr,
            Err(_) => "Not initialized".to_string()
        }
    };

    view! {
        <div class="miner-page">
            <h2>"Miner"</h2>
            
            <div class="miner-content">
                <div class="miner-status">
                    <h3>"Mining Status"</h3>
                    <div class="status-info">
                        <p>"Wallet: " {wallet_address}</p>
                        // Add more mining status information here
                    </div>
                </div>

                <div class="mining-controls">
                    <h3>"Controls"</h3>
                    // Add mining control buttons and options here
                </div>

                <div class="mining-stats">
                    <h3>"Statistics"</h3>
                    // Add mining statistics here
                </div>
            </div>
        </div>
    }
}
