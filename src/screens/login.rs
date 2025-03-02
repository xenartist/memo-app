use egui::{CentralPanel, Context, Vec2, FontId, TextStyle, Frame, TextEdit, RichText};
use super::Screen;
use std::fs;
use crate::encrypt;

pub struct LoginScreen {
    // Password for unlocking wallet
    password: String,
    // Status message (for error display)
    status_message: String,
    // Flag to indicate if wallet file exists
    wallet_exists: bool,
}

impl Default for LoginScreen {
    fn default() -> Self {
        // Check if wallet file exists
        let wallet_exists = Self::check_wallet_file_exists();
        
        Self {
            password: String::new(),
            status_message: String::new(),
            wallet_exists,
        }
    }
}

impl LoginScreen {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Check if wallet file exists
    fn check_wallet_file_exists() -> bool {
        // Get the executable directory
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let wallet_path = exe_dir.join("wallets").join("memo-encrypted.wallet");
                return wallet_path.exists();
            }
        }
        false
    }
    
    // Decrypt wallet file and get seed phrase
    pub fn decrypt_wallet(&self) -> Result<String, String> {
        // Get the executable directory
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| "Failed to get executable directory".to_string())?;
        
        // Get wallet file path
        let wallet_path = exe_dir.join("wallets").join("memo-encrypted.wallet");
        
        // Read encrypted data from file
        let encrypted_data = fs::read_to_string(wallet_path)
            .map_err(|e| format!("Failed to read wallet file: {}", e))?;
        
        // Decrypt data
        let seed_phrase = encrypt::decrypt(&encrypted_data, &self.password)
            .map_err(|e| format!("Decryption error: {}", e))?;
        
        Ok(seed_phrase)
    }

    pub fn render(&mut self, ctx: &Context) -> Option<Screen> {
        let mut next_screen = None;

        // Set font style
        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(22.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(32.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(22.0, egui::FontFamily::Proportional)
        );
        ctx.set_style(style);

        CentralPanel::default().show(ctx, |ui| {
            // Get available size
            let available_size = ui.available_size();
            
            // Calculate vertical position to center the form
            let form_height = if self.wallet_exists { 250.0 } else { 300.0 }; // Approximate height of our form
            let vertical_margin = (available_size.y - form_height) / 2.0;
            if vertical_margin > 0.0 {
                ui.add_space(vertical_margin);
            }
            
            // Center content horizontally
            ui.vertical_centered(|ui| {
                // Set a fixed width for the form
                let form_width = 400.0;
                
                // Create a frame for the form with fixed width
                Frame::group(ui.style())
                    .inner_margin(30.0)
                    .outer_margin(10.0)
                    .show(ui, |ui| {
                        // Set the maximum width for the content
                        ui.set_max_width(form_width);
                        
                        ui.vertical_centered(|ui| {
                            ui.heading("Welcome to Memo World");
                            ui.add_space(30.0);
                            
                            if self.wallet_exists {
                                // Wallet exists, show unlock form
                                ui.label(RichText::new("Enter password to unlock your wallet:").size(22.0));
                                ui.add_space(10.0);
                                
                                // Password field
                                ui.add(TextEdit::singleline(&mut self.password)
                                    .password(true)
                                    .hint_text("Enter password")
                                    .desired_width(300.0)
                                    .font(FontId::new(22.0, egui::FontFamily::Proportional)));
                                
                                ui.add_space(20.0);
                                
                                // Make button a bit larger
                                let button_size = Vec2::new(250.0, 50.0);
                                
                                if ui.add_sized(button_size, egui::Button::new("Unlock Wallet")).clicked() {
                                    // Try to decrypt wallet
                                    match self.decrypt_wallet() {
                                        Ok(_) => {
                                            // Return seed phrase with MainScreen
                                            return next_screen = Some(Screen::MainScreen);
                                        }
                                        Err(e) => {
                                            self.status_message = format!("Error: {}", e);
                                        }
                                    }
                                }
                                
                                // Display status message if any
                                if !self.status_message.is_empty() {
                                    ui.add_space(10.0);
                                    ui.colored_label(
                                        egui::Color32::RED,
                                        &self.status_message
                                    );
                                }
                            } else {
                                // No wallet exists, show options to create or import
                                // Make buttons a bit larger
                                let button_size = Vec2::new(250.0, 60.0);
                                
                                if ui.add_sized(button_size, egui::Button::new("New Wallet")).clicked() {
                                    next_screen = Some(Screen::NewWallet);
                                }
                                
                                ui.add_space(20.0);
                                
                                if ui.add_sized(button_size, egui::Button::new("Import Wallet")).clicked() {
                                    next_screen = Some(Screen::ImportWallet);
                                }
                            }
                        });
                    });
            });
        });

        next_screen
    }
    
    // Get the decrypted seed phrase
    pub fn get_seed_phrase(&self) -> Result<String, String> {
        self.decrypt_wallet()
    }
} 