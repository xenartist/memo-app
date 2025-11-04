use leptos::*;
use crate::core::session::Session;

#[component]
pub fn HomePage(
    session: RwSignal<Session>,
) -> impl IntoView {
    view! {
        <div class="home-page">
            <div class="home-header">
                <h1 class="home-title">
                    <span class="emoji">"🔥"</span>
                    " Welcome to MEMO"
                    <span class="emoji">"📝"</span>
                </h1>
                <p class="home-subtitle">"Memories Engraved, Moments Eternal, Onchain"</p>
            </div>
            
            <div class="home-content">
                <div class="welcome-section">
                    <div class="feature-card">
                        <div class="feature-icon">
                            <i class="fas fa-coins"></i>
                        </div>
                        <h3>"Mint MEMO Tokens"</h3>
                        <p>"Engrave your memories onchain and mint unique MEMO tokens"</p>
                    </div>
                    
                    <div class="feature-card">
                        <div class="feature-icon">
                            <i class="fas fa-fire"></i>
                        </div>
                        <h3>"Burn for Glory"</h3>
                        <p>"Burn MEMO tokens to join the global glory collection"</p>
                    </div>
                    
                    <div class="feature-card">
                        <div class="feature-icon">
                            <i class="fas fa-user-circle"></i>
                        </div>
                        <h3>"Your Profile"</h3>
                        <p>"Create and manage your onchain identity"</p>
                    </div>
                </div>
                
                <div class="info-section">
                    <p class="info-text">
                        "Use the navigation menu to explore different features."
                    </p>
                    {move || {
                        if !session.get().has_user_profile() {
                            view! {
                                <p class="warning-text">
                                    "⚠️ Please create your profile first to start using MEMO."
                                </p>
                            }
                        } else {
                            view! {
                                <p class="success-text">
                                    "✅ You're all set! Start minting and burning MEMO tokens."
                                </p>
                            }
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
