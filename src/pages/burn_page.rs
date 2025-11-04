use leptos::*;
use crate::core::session::Session;
use crate::pages::burn_form::BurnForm;
use crate::pages::memo_card::MemoDetails;
use crate::pages::memo_card_details::MemoCardDetails;

#[component]
pub fn BurnPage(
    session: RwSignal<Session>
) -> impl IntoView {
    // add signal to control burn form visibility
    let (show_burn_form, set_show_burn_form) = create_signal(false);
    
    // add signal to control details modal visibility
    let (show_details_modal, set_show_details_modal) = create_signal(false);
    let (current_memo_details, _set_current_memo_details) = create_signal(Option::<MemoDetails>::None);


    // Optional callbacks for burn events
    let on_burn_success = Callback::new(move |data: (String, u64)| {
        let (_signature, tokens_burned) = data;
        log::info!("Burn successful on page level: {} tokens burned", tokens_burned);
        // close the burn form modal
        set_show_burn_form.set(false);
    });

    let on_burn_error = Callback::new(move |error: String| {
        log::error!("Burn error on page level: {}", error);
    });

    view! {
        <div class="burn-page">
            <div class="burn-page-header">
                // Action buttons
                <div class="burn-actions">
                    <button 
                        class="open-burn-form-btn"
                        on:click=move |_| set_show_burn_form.set(true)
                        disabled=move || !session.get().has_user_profile()
                    >
                        "🔥 Burn MEMO"
                    </button>
                    
                    // Show warning when no profile
                    <Show when=move || !session.get().has_user_profile()>
                        <div class="no-profile-warning">
                            <p>"⚠️ Please create your profile in the Profile page before you can start burning."</p>
                        </div>
                    </Show>
                </div>
            </div>
            
            // Main content area
            <div class="burn-content">
                <div class="header-section" style="text-align: center; padding: 3rem 2rem; color: #666;">
                    <div style="font-size: 1.2rem; margin-bottom: 1rem;">
                        <i class="fas fa-fire" style="margin-right: 8px; color: #dc3545;"></i>
                        "Burn Your MEMO Tokens"
                    </div>
                    <p>"Use the button above to burn your MEMO tokens."</p>
                    <p style="margin-top: 1rem; font-size: 0.9em; color: #999;">"Note: Burn history is stored on-chain. Local storage has been removed."</p>
                </div>
            </div>
            
            // Modal overlay for burn form
            <Show when=move || show_burn_form.get()>
                <div class="modal-overlay" on:click=move |_| set_show_burn_form.set(false)>
                    <div class="modal-content" on:click=|e| e.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Burn MEMO"</h3>
                            <button 
                                class="modal-close-btn"
                                on:click=move |_| set_show_burn_form.set(false)
                                title="Close"
                            >
                                "×"
                            </button>
                        </div>
                        
                        <div class="modal-body">
                            {
                                let success_cb = on_burn_success.clone();
                                let error_cb = on_burn_error.clone();
                                
                                view! {
                                    <BurnForm 
                                        session=session 
                                        on_burn_success=success_cb
                                        on_burn_error=error_cb
                                    />
                                }
                            }
                        </div>
                    </div>
                </div>
            </Show>

            // use details modal component (without burn button for already burned items)
            <MemoCardDetails 
                show_modal=show_details_modal.into()
                set_show_modal=set_show_details_modal
                memo_details=current_memo_details.into()
                session=session
                on_close=Callback::new(move |_| {
                    log::info!("Burn details modal closed");
                })
            />
        </div>
    }
} 