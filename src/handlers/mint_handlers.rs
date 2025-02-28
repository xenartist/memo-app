use dioxus::prelude::*;
use crate::session;
use log;

// Show mint modal
pub fn show_mint_modal(
    mut error_message: Signal<String>,
    mut show_modal: Signal<bool>,
    session_active: Signal<bool>
) -> impl FnMut(MouseEvent) {
    move |_| {
        // Reset error message
        error_message.set("".to_string());
        
        // Show the modal
        show_modal.set(true);
        
        // Check if session is active and log it
        log::info!("Mint button clicked, showing mint modal. Session active: {}", session_active.read());
    }
}

// Close mint modal
pub fn close_mint_modal(mut show_modal: Signal<bool>) -> impl FnMut(MouseEvent) {
    move |_| {
        show_modal.set(false);
    }
}

// Process mint operation
pub fn process_mint(
    mut error_message: Signal<String>,
    mut is_minting: Signal<bool>,
    mut show_modal: Signal<bool>,
    session_active: Signal<bool>,
    memo_name: Signal<String>,
    memo_description: Signal<String>,
) -> impl FnMut(MouseEvent) {
    move |_| {
        // Reset error message
        error_message.set("".to_string());
        
        // Check if session is active
        if !*session_active.read() {
            error_message.set("Wallet session is not active. Please unlock your wallet first.".to_string());
            return;
        }
        
        // Validate input
        let name = memo_name.read().clone();
        if name.is_empty() {
            error_message.set("Memo name cannot be empty".to_string());
            return;
        }
        
        // Get description
        let description = memo_description.read().clone();
        
        // Set minting state to true
        is_minting.set(true);
        
        // Simulate minting process (would be replaced with actual API call)
        // In a real implementation, this would be an async operation
        // For now, we'll just simulate a delay with a timeout
        
        // This is a placeholder for the actual minting logic
        // In a real app, you would call your blockchain API here
        
        // For demo purposes, we'll just log the minting and update the UI after a delay
        log::info!("Memo '{}' minted successfully: {}", name, description);
        error_message.set(format!("Memo '{}' minted successfully!", name));
        
        // Reset states after "minting"
        is_minting.set(false);
        show_modal.set(false);
    }
} 