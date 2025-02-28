use dioxus::prelude::*;

/// Modal dialog component for displaying wallet mnemonic
#[component]
pub fn MnemonicModal(
    mnemonic: String,
    visible: bool,
    on_close: EventHandler<()>,
) -> Element {
    if !visible {
        return rsx!{ Fragment {} };
    }

    // Split the mnemonic into words and add index
    let words: Vec<(usize, &str)> = mnemonic
        .split_whitespace()
        .enumerate()
        .map(|(i, word)| (i + 1, word))
        .collect();

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-content",
                // Prevent clicks inside the modal from closing it
                onclick: move |evt| evt.stop_propagation(),
                h2 { "Your Recovery Phrase" }
                p { class: "warning", "Write these words down and keep them in a safe place. They are the only way to recover your wallet!" }
                
                div {
                    class: "mnemonic-grid",
                    for (index, word) in words {
                        div {
                            class: "mnemonic-word",
                            span { class: "word-number", "{index}." }
                            span { class: "word-text", "{word}" }
                        }
                    }
                }
                
                div {
                    class: "modal-actions",
                    button {
                        class: "modal-button",
                        onclick: move |_| {
                            log::info!("Save Wallet button clicked in modal");
                            on_close.call(())
                        },
                        "Save Wallet"
                    }
                }
            }
        }
    }
} 