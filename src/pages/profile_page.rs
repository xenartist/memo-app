use leptos::*;
use log;
use crate::core::session::{Session, UserProfile, parse_user_profile};
use crate::core::rpc::RpcConnection;

#[component]
pub fn ProfilePage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (loading, set_loading) = create_signal(false);  // default to not show loading state
    let (user_profile, set_user_profile) = create_signal::<Option<UserProfile>>(None);

    // fetch profile information
    spawn_local(async move {
        // check if session has cached profile
        if let Some(profile) = session.get().get_user_profile() {
            set_user_profile.set(Some(profile));
            return; // if there is cached data, return immediately without requesting network
        }

        // if there is no cached data, show loading state
        set_loading.set(true);
        
        let mut current_session = session.get();
        if let Ok(pubkey) = current_session.get_public_key() {
            log::info!("Fetching profile for pubkey: {}", pubkey);
            
            let rpc = RpcConnection::new();
            match rpc.get_user_profile(&pubkey).await {
                Ok(result) => {
                    if let Ok(profile) = parse_user_profile(&result) {
                        log::info!("Successfully fetched user profile");
                        current_session.set_user_profile(Some(profile.clone()));
                        session.set(current_session);
                        set_user_profile.set(Some(profile));
                    }
                }
                Err(e) => {
                    log::error!("Failed to get user profile: {:?}", e);
                }
            }
        }
        set_loading.set(false);
    });

    view! {
        <div class="profile-page">
            <h2>"User Profile"</h2>
            
            // main content area
            <div class="profile-content">
                {move || match user_profile.get() {
                    Some(profile) => view! {
                        <div class="profile-form">
                            <div class="form-group">
                                <label>"Username"</label>
                                <p class="profile-value">{profile.username}</p>
                            </div>
                            <div class="form-group">
                                <label>"Public Key"</label>
                                <p class="profile-value pubkey">{profile.pubkey}</p>
                            </div>
                            <div class="profile-stats">
                                <div class="stat-item">
                                    <label>"Total Minted"</label>
                                    <p class="stat-value">{profile.total_minted}</p>
                                </div>
                                <div class="stat-item">
                                    <label>"Total Burned"</label>
                                    <p class="stat-value">{profile.total_burned}</p>
                                </div>
                                <div class="stat-item">
                                    <label>"Mint Count"</label>
                                    <p class="stat-value">{profile.mint_count}</p>
                                </div>
                                <div class="stat-item">
                                    <label>"Burn Count"</label>
                                    <p class="stat-value">{profile.burn_count}</p>
                                </div>
                            </div>
                        </div>
                    },
                    None => view! {
                        <div class="create-profile-form">
                            {move || if loading.get() {
                                view! {
                                    <div class="loading-state">
                                        <p>"Checking profile information..."</p>
                                    </div>
                                }
                            } else {
                                view! {
                                    <div>
                                        <p class="info-text">
                                            "You haven't created a profile yet. Create one to start using all features."
                                        </p>
                                        // TODO: add profile creation form
                                    </div>
                                }
                            }}
                        </div>
                    }
                }}
            </div>
        </div>
    }
} 