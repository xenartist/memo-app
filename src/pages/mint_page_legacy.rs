use leptos::*;
use crate::core::session::Session;
use wasm_bindgen::JsCast;
use crate::pages::mint_form::MintForm;
use crate::pages::memo_card::MemoDetails;
use crate::pages::memo_card_details::MemoCardDetails;
use crate::pages::burn_onchain::{BurnOptions, BurnOnchain};
use std::rc::Rc;

#[component]
pub fn MintPage(
    session: RwSignal<Session>
) -> impl IntoView {
    // add signal to control mint form visibility
    let (show_mint_form, set_show_mint_form) = create_signal(false);
    
    // add signal to control details modal visibility
    let (show_details_modal, set_show_details_modal) = create_signal(false);
    let (current_memo_details, _set_current_memo_details) = create_signal(Option::<MemoDetails>::None);
    
    // ✅ add burn onchain related states
    let (show_burn_onchain, set_show_burn_onchain) = create_signal(false);
    let (burn_signature, _set_burn_signature) = create_signal(String::new());


    // Optional callbacks for mint events
    let on_mint_success = Rc::new(move |_signature: String, tokens_minted: u64, total_minted: u64| {
        log::info!("Mint successful on page level: {} tokens minted, total: {}", tokens_minted, total_minted);
    });

    let on_mint_error = Rc::new(move |error: String| {
        log::error!("Mint error on page level: {}", error);
    });

    view! {
        <div class="mint-page">
            <div class="mint-page-header">
                // Action buttons
                <div class="mint-actions">
                    <button 
                        class="open-mint-form-btn"
                        on:click=move |_| set_show_mint_form.set(true)
                        disabled=move || !session.get().has_user_profile()
                    >
                        "🚀 Engrave & Mint"
                    </button>
                    
                    // Show warning when no profile
                    <Show when=move || !session.get().has_user_profile()>
                        <div class="no-profile-warning">
                            <p>"⚠️ Please create your mint profile in the Profile page before you can start minting."</p>
                        </div>
                    </Show>
                </div>
            </div>
            
            // Main content area
            <div class="mint-content">
                <div class="header-section" style="text-align: center; padding: 3rem 2rem; color: #666;">
                    <div style="font-size: 1.2rem; margin-bottom: 1rem;">
                        <i class="fas fa-coins" style="margin-right: 8px; color: #28a745;"></i>
                        "Mint Your MEMO Tokens"
                    </div>
                    <p>"Use the button above to engrave your memories and mint MEMO tokens."</p>
                    <p style="margin-top: 1rem; font-size: 0.9em; color: #999;">"Note: Mint history is stored on-chain. Local storage has been removed."</p>
                </div>
            </div>
            
            // Modal overlay for mint form
            <Show when=move || show_mint_form.get()>
                <div class="modal-overlay" on:click=move |ev| {
                    // if click on overlay itself, close the form
                    if let Some(target) = ev.target() {
                        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                            if element.class_list().contains("modal-overlay") {
                                set_show_mint_form.set(false);
                            }
                        }
                    }
                }>
                    <div class="mint-form-container">
                        <MintForm
                            session=session
                            class="mint-form-in-modal"
                            on_mint_success=on_mint_success.clone()
                            on_mint_error=on_mint_error.clone()
                            on_close=Rc::new(move || set_show_mint_form.set(false))
                        />
                    </div>
                </div>
            </Show>

            // use details modal component (with burn button for mint records)
            <MemoCardDetails 
                show_modal=show_details_modal.into()
                set_show_modal=set_show_details_modal
                memo_details=current_memo_details.into()
                session=session
                on_burn_choice=Callback::new(move |(signature, burn_options): (String, BurnOptions)| {
                    log::info!("Burn choice made from mint page for signature: {}, options: {:?}", signature, burn_options);
                    // TODO: implement burn handling logic if needed
                })
                on_close=Callback::new(move |_| {
                    log::info!("Details modal closed");
                })
            />

            // ✅ add BurnOnchain Modal
            <BurnOnchain
                show_modal=show_burn_onchain.into()
                set_show_modal=set_show_burn_onchain
                signature=burn_signature.into()
                memo_details=current_memo_details.into()
                session=session
                on_burn_choice=Callback::new(move |(sig, burn_options): (String, BurnOptions)| {
                    log::info!("Burn onchain choice made from mint page for signature: {}, options: {:?}", sig, burn_options);
                    // burn success, no need to refresh mint records
                })
                on_close=Callback::new(move |_| {
                    log::info!("Burn onchain modal closed from mint page");
                    set_show_burn_onchain.set(false);
                })
            />
        </div>
    }
}
