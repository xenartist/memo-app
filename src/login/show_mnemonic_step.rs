use leptos::*;
use crate::CreateWalletStep;
use crate::core::NetworkType;

#[component]
pub fn ShowMnemonicStep(
    set_current_step: WriteSignal<CreateWalletStep>,
    set_mnemonic: WriteSignal<String>,
    selected_network: RwSignal<NetworkType>,
) -> impl IntoView {
    let (word_count, set_word_count) = create_signal(12); // default 12 words
    let (current_mnemonic, set_current_mnemonic) = create_signal(String::new());

    // generate mnemonic
    let generate_mnemonic = move || {
        if let Ok(new_mnemonic) = crate::core::wallet::generate_mnemonic(word_count.get()) {
            set_mnemonic.set(new_mnemonic.clone());
            set_current_mnemonic.set(new_mnemonic);
        }
    };

    // generate mnemonic
    generate_mnemonic();

    // when word count changes, generate mnemonic
    let handle_word_count_change = move |new_count: u32| {
        set_word_count.set(new_count);
        generate_mnemonic();
    };

    view! {
        <div class="login-container">
            <div class="header-with-back">
                <button 
                    class="back-btn"
                    on:click=move |_| set_current_step.set(CreateWalletStep::Initial)
                >
                    "← Back"
                </button>
                <h2>"Create Your Mnemonic Phrase"</h2>
            </div>
            
            // Display selected network (read-only)
            <div class="info-message" style="margin: 1rem auto; max-width: 500px;">
                <i class="fas fa-network-wired"></i>
                <span>
                    "Network: "
                    {move || match selected_network.get() {
                        NetworkType::Testnet => "Testnet",
                        NetworkType::ProdStaging => "Prod Staging",
                        NetworkType::Mainnet => "Mainnet",
                    }}
                </span>
            </div>
            
            <div class="word-count-selector">
                <div class="radio-group">
                    <label>
                        <input 
                            type="radio"
                            name="word-count"
                            checked=move || word_count.get() == 12
                            on:change=move |_| handle_word_count_change(12)
                        />
                        <i class="fas fa-shield-alt"></i>
                        " 12 Words"
                    </label>
                    <label>
                        <input 
                            type="radio"
                            name="word-count"
                            checked=move || word_count.get() == 24
                            on:change=move |_| handle_word_count_change(24)
                        />
                        <i class="fas fa-shield"></i>
                        " 24 Words"
                    </label>
                </div>
                <p class="word-count-hint">
                    <i class="fas fa-info-circle"></i>
                    " "
                    {move || if word_count.get() == 12 {
                        "12 words provide standard security"
                    } else {
                        "24 words provide maximum security"
                    }}
                </p>
            </div>

            <div class="warning-message" style="margin: 1.5rem auto;">
                <i class="fas fa-exclamation-triangle"></i>
                <span>"Write down these words in order and keep them safe. Never share them with anyone!"</span>
            </div>

            <div class="mnemonic-display">
                <i class="fas fa-lock" style="margin-right: 0.5rem;"></i>
                {current_mnemonic}
            </div>

            <div class="button-group">
                <button class="wallet-btn" 
                    on:click=move |_| set_current_step.set(
                        CreateWalletStep::VerifyMnemonic(current_mnemonic.get())
                    )>
                    <i class="fas fa-check-circle"></i>
                    " I've Written It Down"
                </button>
            </div>
        </div>
    }
}