use leptos::*;
use crate::core::session::Session;

#[component]
pub fn BurnPage(
    session: RwSignal<Session>
) -> impl IntoView {
    view! {
        <div class="burn-page">
            <div class="burn-page-header">
                <h2>"🔥 Burn MEMO Tokens"</h2>
                <p>"Burn MEMO Tokens"</p>
            </div>
            
            <div class="burn-content">
                <div class="coming-soon-message">
                    <div class="icon">
                        <i class="fas fa-fire" style="font-size: 3rem; color: #dc3545; margin-bottom: 1rem;"></i>
                    </div>
                    <h3>"Coming Soon..."</h3>
                    <p>"Burn function is under development, stay tuned!"</p>
                </div>
            </div>
        </div>
    }
} 