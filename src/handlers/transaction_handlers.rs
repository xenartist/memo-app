use dioxus::prelude::*;
use crate::wallet::{Transaction, SignedTransaction};
use crate::config;
use crate::session;
use std::rc::Rc;
use std::cell::RefCell;

// Show send transaction modal
pub fn show_send_transaction_modal(
    mut recipient_address: Signal<String>,
    mut send_amount: Signal<String>,
    mut transaction_memo: Signal<String>,
    mut error_message: Signal<String>,
    mut show_send_modal: Signal<bool>,
    mut session_active: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        recipient_address.set(String::new());
        send_amount.set(String::new());
        transaction_memo.set(String::new());
        error_message.set(String::new());
        show_send_modal.set(true);
        
        // Check if session is active
        session_active.set(session::is_session_active());
        
        log::info!("Send button clicked, showing send modal. Session active: {}", session_active.read());
    }
}

// Prepare transaction
pub fn prepare_transaction(
    recipient_address: Signal<String>,
    send_amount: Signal<String>,
    transaction_memo: Signal<String>,
    mut error_message: Signal<String>,
    wallet_address: Signal<String>,
    mut prepared_transaction: Signal<Option<Transaction>>,
    wallet_balance: Signal<Option<f64>>,
    mut transaction_password: Signal<String>,
    mut show_send_modal: Signal<bool>,
    mut show_transaction_password_modal: Signal<bool>,
    sign_and_send_transaction: Rc<RefCell<dyn FnMut(Option<&str>)>>,
    session_active: Signal<bool>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        // Validate inputs
        let to_address = recipient_address.read().clone();
        if to_address.is_empty() {
            error_message.set("Recipient address cannot be empty".to_string());
            return;
        }
        
        let amount_str = send_amount.read().clone();
        let amount = match amount_str.parse::<f64>() {
            Ok(a) if a > 0.0 => a,
            _ => {
                error_message.set("Invalid amount".to_string());
                return;
            }
        };
        
        let memo = transaction_memo.read().clone();
        let memo = if memo.is_empty() { None } else { Some(memo) };
        
        // Get sender address
        let from_address = wallet_address.read().clone();
        
        // Create transaction
        let transaction = Transaction {
            from: from_address,
            to: to_address,
            amount,
            memo,
        };
        
        prepared_transaction.set(Some(transaction));
        
        // If session is active, proceed directly to signing
        // Otherwise, show password modal
        let session_active_value = session_active.read().clone();
        if session_active_value {
            sign_and_send_transaction.borrow_mut()(None::<&str>);
        } else {
            transaction_password.set(String::new());
            show_send_modal.set(false);
            show_transaction_password_modal.set(true);
        }
    }
}

// Confirm transaction with password
pub fn confirm_transaction_with_password(
    transaction_password: Signal<String>,
    mut error_message: Signal<String>,
    sign_and_send_transaction: Rc<RefCell<dyn FnMut(Option<&str>)>>,
) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        let password = transaction_password.read().clone();
        
        if password.is_empty() {
            error_message.set("Password cannot be empty".to_string());
            return;
        }
        
        sign_and_send_transaction.borrow_mut()(Some(&password));
    }
}

// Create sign and send transaction function
pub fn create_sign_and_send_transaction(
    mut is_sending: Signal<bool>,
    mut error_message: Signal<String>,
    prepared_transaction: Signal<Option<Transaction>>,
    mut show_transaction_password_modal: Signal<bool>,
    mut show_send_modal: Signal<bool>,
    mut session_active: Signal<bool>,
) -> impl FnMut(Option<&str>) {
    move |password: Option<&str>| {
        is_sending.set(true);
        error_message.set(String::new());
        
        if let Some(tx) = prepared_transaction.read().as_ref() {
            log::info!("Signing transaction to send {} {} to {}", tx.amount, config::TOKEN_SYMBOL, tx.to);
            
            // Sign the transaction
            match crate::wallet::sign_transaction(tx, password) {
                Ok(signed_tx) => {
                    // In a real app, you would send the transaction to the network here
                    log::info!("Transaction signed successfully: {:?}", signed_tx);
                    
                    // For demo purposes, just log the transaction and show success
                    is_sending.set(false);
                    show_transaction_password_modal.set(false);
                    show_send_modal.set(false);
                    
                    // Show success message
                    error_message.set(format!("Transaction of {} {} to {} sent successfully!", 
                        tx.amount, config::TOKEN_SYMBOL, tx.to));
                        
                    // Update session active state
                    session_active.set(session::is_session_active());
                },
                Err(err) => {
                    log::error!("Failed to sign transaction: {}", err);
                    error_message.set(format!("Failed to sign transaction: {}", err));
                    is_sending.set(false);
                }
            }
        } else {
            error_message.set("No transaction prepared".to_string());
            is_sending.set(false);
        }
    }
}

// Close send modal
pub fn close_send_modal(mut show_send_modal: Signal<bool>) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        show_send_modal.set(false);
    }
}

// Close transaction password modal
pub fn close_transaction_password_modal(mut show_transaction_password_modal: Signal<bool>) -> impl FnMut(MouseEvent) {
    move |_: MouseEvent| {
        show_transaction_password_modal.set(false);
    }
} 