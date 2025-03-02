use egui::{CentralPanel, Context, FontId, TextStyle, Frame, Vec2, TextEdit, Grid, RadioButton, ScrollArea};
use super::Screen;
use std::fs::{self, File};
use std::io::Write;
use crate::encrypt;
use bip39::Mnemonic;

pub struct ImportWalletScreen {
    // Whether to use 24 words (otherwise use 12)
    use_24_words: bool,
    // Seed words input by user
    seed_words: Vec<String>,
    // Password for encrypting the wallet
    password: String,
    // Confirm password
    confirm_password: String,
    // Status message
    status_message: String,
}

impl Default for ImportWalletScreen {
    fn default() -> Self {
        // Initialize with empty seed words for 12 words (default)
        let mut seed_words = Vec::new();
        for _ in 0..12 {
            seed_words.push(String::new());
        }
        
        Self {
            use_24_words: false,
            seed_words,
            password: String::new(),
            confirm_password: String::new(),
            status_message: String::new(),
        }
    }
}

impl ImportWalletScreen {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Update the number of seed word inputs based on selection
    fn update_seed_word_count(&mut self) {
        let target_count = if self.use_24_words { 24 } else { 12 };
        
        // Resize the vector to match the target count
        if self.seed_words.len() < target_count {
            // Add empty strings if we need more inputs
            while self.seed_words.len() < target_count {
                self.seed_words.push(String::new());
            }
        } else if self.seed_words.len() > target_count {
            // Remove excess inputs if we need fewer
            self.seed_words.truncate(target_count);
        }
    }
    
    // Validate the mnemonic phrase
    fn validate_mnemonic(&self) -> Result<String, String> {
        // Join the seed words
        let phrase = self.seed_words.join(" ");
        
        // Check if any words are empty
        if self.seed_words.iter().any(|word| word.trim().is_empty()) {
            return Err("All seed words must be filled".to_string());
        }
        
        // Validate the mnemonic
        match Mnemonic::parse_normalized(&phrase) {
            Ok(_) => Ok(phrase),
            Err(e) => Err(format!("Invalid mnemonic: {}", e)),
        }
    }
    
    // Save wallet to file
    fn save_wallet(&self, seed_phrase: &str, password: &str) -> Result<(), String> {
        // Encrypt the seed phrase
        let encrypted_data = encrypt::encrypt(seed_phrase, password)
            .map_err(|e| format!("Encryption error: {}", e))?;
        
        // Get the executable directory
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| "Failed to get executable directory".to_string())?;
        
        // Create wallets directory if it doesn't exist
        let wallets_dir = exe_dir.join("wallets");
        fs::create_dir_all(&wallets_dir)
            .map_err(|e| format!("Failed to create wallets directory: {}", e))?;
        
        // Create wallet file
        let wallet_path = wallets_dir.join("memo-encrypted.wallet");
        let mut file = File::create(wallet_path)
            .map_err(|e| format!("Failed to create wallet file: {}", e))?;
        
        // Write encrypted data to file
        file.write_all(encrypted_data.as_bytes())
            .map_err(|e| format!("Failed to write to wallet file: {}", e))?;
        
        Ok(())
    }
    
    // Get the seed phrase
    pub fn get_seed_phrase(&self) -> String {
        self.seed_words.join(" ")
    }
    
    // Display seed word input fields in a grid
    fn show_seed_word_inputs(&mut self, ui: &mut egui::Ui) {
        let words_per_row = if self.use_24_words { 6 } else { 4 };
        let rows = self.seed_words.len() / words_per_row;
        
        // Create a grid for the seed word inputs
        Grid::new("seed_words_grid")
            .num_columns(words_per_row)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                for row in 0..rows {
                    for col in 0..words_per_row {
                        let index = row * words_per_row + col;
                        let word_number = index + 1; // 1-indexed for display
                        
                        ui.vertical(|ui| {
                            ui.label(format!("{}.", word_number));
                            ui.add(TextEdit::singleline(&mut self.seed_words[index])
                                .hint_text("word")
                                .desired_width(100.0));
                        });
                    }
                    ui.end_row();
                }
            });
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
            FontId::new(36.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(22.0, egui::FontFamily::Proportional)
        );
        ctx.set_style(style);

        CentralPanel::default().show(ctx, |ui| {
            // Add some vertical space at the top
            ui.add_space(20.0);
            
            // Use a scroll area to handle content that might not fit
            ScrollArea::vertical().show(ui, |ui| {
                // Create a centered form
                ui.vertical_centered(|ui| {
                    ui.heading("Import Existing Wallet");
                    ui.add_space(20.0);
                    
                    // Set a fixed width for the form
                    let form_width = 600.0;
                    
                    // Create a frame for the form with fixed width
                    Frame::group(ui.style())
                        .inner_margin(30.0)
                        .outer_margin(10.0)
                        .show(ui, |ui| {
                            // Set the maximum width for the content
                            ui.set_max_width(form_width);
                            
                            ui.vertical_centered(|ui| {
                                // Radio buttons to select 12 or 24 words
                                ui.label("Recovery seed length:");
                                ui.add_space(5.0);
                                
                                ui.horizontal(|ui| {
                                    let mut changed = false;
                                    
                                    if ui.add(RadioButton::new(!self.use_24_words, "12 words")).clicked() {
                                        if self.use_24_words {
                                            self.use_24_words = false;
                                            changed = true;
                                        }
                                    }
                                    
                                    ui.add_space(20.0);
                                    
                                    if ui.add(RadioButton::new(self.use_24_words, "24 words")).clicked() {
                                        if !self.use_24_words {
                                            self.use_24_words = true;
                                            changed = true;
                                        }
                                    }
                                    
                                    // Update seed word inputs if count changes
                                    if changed {
                                        self.update_seed_word_count();
                                    }
                                });
                                
                                ui.add_space(20.0);
                                
                                ui.label("Enter your recovery seed phrase:");
                                ui.add_space(10.0);
                                
                                // Display seed word input fields
                                self.show_seed_word_inputs(ui);
                                
                                ui.add_space(20.0);
                                
                                // Password fields for encryption
                                ui.label("Enter password to encrypt your wallet:");
                                ui.add_space(5.0);
                                
                                // Password field
                                ui.horizontal(|ui| {
                                    ui.label("Password:      ");
                                    ui.add(TextEdit::singleline(&mut self.password)
                                        .password(true)
                                        .hint_text("Enter password")
                                        .desired_width(300.0));
                                });
                                
                                ui.add_space(5.0);
                                
                                // Confirm password field
                                ui.horizontal(|ui| {
                                    ui.label("Confirm:       ");
                                    ui.add(TextEdit::singleline(&mut self.confirm_password)
                                        .password(true)
                                        .hint_text("Confirm password")
                                        .desired_width(300.0));
                                });
                                
                                // Display status message if any
                                if !self.status_message.is_empty() {
                                    ui.add_space(10.0);
                                    ui.colored_label(
                                        if self.status_message.starts_with("Error") { 
                                            egui::Color32::RED 
                                        } else { 
                                            egui::Color32::GREEN 
                                        },
                                        &self.status_message
                                    );
                                }
                            });
                            
                            ui.add_space(20.0);
                            
                            // Bottom buttons
                            ui.horizontal(|ui| {
                                // Calculate button sizes and spacing
                                let button_size = Vec2::new(200.0, 40.0);
                                let available_width = ui.available_width();
                                let spacing = (available_width - 2.0 * button_size.x) / 3.0;
                                
                                ui.add_space(spacing);
                                
                                if ui.add_sized(button_size, egui::Button::new("Back")).clicked() {
                                    next_screen = Some(Screen::Login);
                                }
                                
                                ui.add_space(spacing);
                                
                                if ui.add_sized(button_size, egui::Button::new("Import Encrypted Wallet")).clicked() {
                                    // Validate mnemonic
                                    match self.validate_mnemonic() {
                                        Ok(seed_phrase) => {
                                            // Validate passwords
                                            if self.password.is_empty() {
                                                self.status_message = "Error: Password cannot be empty".to_string();
                                            } else if self.password != self.confirm_password {
                                                self.status_message = "Error: Passwords do not match".to_string();
                                            } else {
                                                // Save wallet
                                                match self.save_wallet(&seed_phrase, &self.password) {
                                                    Ok(_) => {
                                                        self.status_message = "Wallet imported successfully!".to_string();
                                                        // Navigate to main screen after successful import
                                                        next_screen = Some(Screen::MainScreen);
                                                    }
                                                    Err(e) => {
                                                        self.status_message = format!("Error: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.status_message = format!("Error: {}", e);
                                        }
                                    }
                                }
                                
                                ui.add_space(spacing);
                            });
                        });
                    
                    ui.add_space(20.0);
                });
            });
        });

        next_screen
    }
} 