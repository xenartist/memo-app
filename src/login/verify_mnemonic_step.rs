use leptos::*;
use crate::CreateWalletStep;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Clone, Debug)]
struct WordState {
    word: String,
    index: usize,
    is_used: bool,
}

#[component]
pub fn VerifyMnemonicStep(
    mnemonic: ReadSignal<String>,
    set_current_step: WriteSignal<CreateWalletStep>
) -> impl IntoView {
    let words: Vec<String> = mnemonic.get().split_whitespace().map(String::from).collect();
    let total_words = words.len();
    let (total_count, _) = create_signal(total_words);

    let mut shuffled_words: Vec<WordState> = words.iter()
        .enumerate()
        .map(|(i, w)| WordState {
            word: w.clone(),
            index: i,
            is_used: false,
        })
        .collect();
    shuffled_words.shuffle(&mut thread_rng());
    
    let (word_states, set_word_states) = create_signal(shuffled_words);
    let (current_index, set_current_index) = create_signal(0);
    let (error_message, set_error_message) = create_signal(String::new());

    view! {
        <div class="login-container">
            <h2>"Verify Your Mnemonic Phrase"</h2>
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
                <For
                    each=move || word_states.get()
                    key=|word| word.word.clone()
                    children=move |word| {
                        let word = word.clone();
                        let word_for_click = word.clone();
                        let total = total_count.get();
                        
                        let on_click = move |_| {
                            if word_for_click.index == current_index.get() {
                                set_word_states.update(|states| {
                                    if let Some(state) = states.iter_mut().find(|s| s.word == word_for_click.word) {
                                        state.is_used = true;
                                    }
                                });
                                set_current_index.update(|i| *i += 1);
                                set_error_message.set(String::new());

                                if current_index.get() == total {
                                    set_current_step.set(CreateWalletStep::SetPassword);
                                }
                            } else {
                                set_error_message.set("Wrong word order. Try again!".to_string());
                            }
                        };

                        view! {
                            <button
                                class="word-button"
                                class:used=move || word.is_used
                                disabled=word.is_used
                                on:click=on_click
                            >
                                {word.word}
                            </button>
                        }
                    }
                />
            </div>

            <div class="progress-bar">
                {move || format!("Progress: {}/{}", current_index.get(), total_count.get())}
            </div>
        </div>
    }
}