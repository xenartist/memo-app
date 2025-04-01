use leptos::*;
use log;
use crate::core::session::{Session, UserProfile, parse_user_profile};
use crate::core::rpc::RpcConnection;

#[component]
pub fn ProfilePage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (loading, set_loading) = create_signal(true);
    let (user_profile, set_user_profile) = create_signal::<Option<UserProfile>>(None);

    // get current profile from session
    create_effect(move |_| {
        if let Some(profile) = session.get().get_user_profile() {
            set_user_profile.set(Some(profile));
            set_loading.set(false);
        }
    });

    // fetch user profile from chain
    spawn_local(async move {
        let mut current_session = session.get();
        if let Ok(pubkey) = current_session.get_public_key() {
            log::info!("Fetching profile for pubkey: {}", pubkey);
            
            let rpc = RpcConnection::new();
            match rpc.get_user_profile(&pubkey).await {
                Ok(result) => {
                    match parse_user_profile(&result) {
                        Ok(profile) => {
                            log::info!("Successfully fetched user profile");
                            // update session cache
                            current_session.set_user_profile(Some(profile.clone()));
                            session.set(current_session);
                            // update UI
                            set_user_profile.set(Some(profile));
                        }
                        Err(e) => {
                            log::error!("Failed to parse user profile: {}", e);
                            // if parsing fails, user may not have created a profile yet
                            set_user_profile.set(None);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get user profile: {}", e);
                    set_user_profile.set(None);
                }
            }
        } else {
            log::error!("Failed to get public key from session");
        }
        set_loading.set(false);
    });

    view! {
        <div class="profile-page">
            {move || {
                if loading.get() {
                    view! {
                        <div class="loading-state">
                            <p>"Loading profile information..."</p>
                        </div>
                    }
                } else {
                    match user_profile.get() {
                        Some(profile) => view! {
                            <div class="profile-form">
                                <h2>"User Profile"</h2>
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
                                <h2>"Create Profile"</h2>
                                <p class="info-text">
                                    "You haven't created a profile yet. Create one to start using all features."
                                </p>
                                // TODO: add profile creation form
                            </div>
                        }
                    }
                }
            }}
        </div>
    }
} 