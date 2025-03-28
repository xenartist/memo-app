use leptos::*;
use crate::CreateWalletStep;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Clone, Debug)]
struct WordState {
    word: String,
    index: usize,
    selected: bool,
}

#[component]
pub fn VerifyMnemonicStep(
    mnemonic: ReadSignal<String>,
    set_current_step: WriteSignal<CreateWalletStep>
) -> impl IntoView {
    let words: Vec<String> = mnemonic.get().split_whitespace().map(String::from).collect();
    let total_words = words.len();
    
    let mut shuffled_words: Vec<WordState> = words.iter()
        .enumerate()
        .map(|(i, w)| WordState {
            word: w.clone(),
            index: i,
            selected: false,
        })
        .collect();
    shuffled_words.shuffle(&mut thread_rng());
    
    let (word_states, set_word_states) = create_signal(shuffled_words);
    let (current_index, set_current_index) = create_signal(0);
    let (error_message, set_error_message) = create_signal(String::new());

    view! {
        <div class="login-container">
            <div class="header-with-back">
                <button 
                    class="back-btn"
                    on:click=move |_| set_current_step.set(CreateWalletStep::ShowMnemonic(mnemonic.get()))
                >
                    "← Back"
                </button>
                <h2>"Verify Your Mnemonic Phrase"</h2>
            </div>
            <p class="verify-instruction">
                "Click the words in the correct order to verify your backup"
            </p>

            <div class="current-word-index">
                {move || format!("Select word #{}", current_index.get() + 1)}
            </div>

            <div class="error-message">
                {move || error_message.get()}
            </div>

            <div class="word-grid">
                {move || {
                    word_states.get().into_iter().map(|word| {
                        let word_for_click = word.clone();
                        
                        let on_click = move |_| {
                            if word_for_click.index == current_index.get() {
                                set_word_states.update(|states| {
                                    if let Some(state) = states.iter_mut().find(|w| w.word == word_for_click.word) {
                                        state.selected = true;
                                    }
                                });
                                set_current_index.update(|i| *i += 1);
                                set_error_message.set(String::new());

                                if current_index.get() == total_words {
                                    set_current_step.set(CreateWalletStep::SetPassword);
                                }
                            } else {
                                set_error_message.set("Wrong word order. Try again!".to_string());
                            }
                        };

                        view! {
                            <button
                                class="word-button"
                                on:click=on_click
                            >
                                {if word.selected { String::new() } else { word.word }}
                            </button>
                        }
                    }).collect_view()
                }}
            </div>

            <div class="progress-bar">
                {move || format!("Progress: {}/{}", current_index.get(), total_words)}
            </div>
        </div>
    }
}