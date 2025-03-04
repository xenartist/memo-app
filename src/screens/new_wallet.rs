use egui::{CentralPanel, Context, FontId, TextStyle, Grid, RadioButton, Ui, ScrollArea, Frame, Vec2, TextEdit};
use super::Screen;
use crate::core::wallet::Wallet;

pub struct NewWalletScreen {
    // Store recovery seed words
    seed_words: Vec<String>,
    // Whether to use 24 words (otherwise use 12)
    use_24_words: bool,
    // Password for encrypting the wallet
    password: String,
    // Confirm password
    confirm_password: String,
    // Status message
    status_message: String,
}

impl Default for NewWalletScreen {
    fn default() -> Self {
        let mut screen = Self {
            seed_words: Vec::new(),
            use_24_words: false,
            password: String::new(),
            confirm_password: String::new(),
            status_message: String::new(),
        };
        // Initialize with 12 words
        screen.generate_seed_words();
        screen
    }
}

impl NewWalletScreen {
    pub fn new() -> Self {
        Self::default()
    }

    // Generate BIP39 mnemonic seed words
    fn generate_seed_words(&mut self) {
        // Determine word count
        let word_count = if self.use_24_words { 24 } else { 12 };
        
        // Generate seed words using the Wallet module
        match Wallet::generate_seed_words(word_count) {
            Ok(words) => self.seed_words = words,
            Err(e) => {
                self.status_message = format!("Error generating seed words: {}", e);
                // Ensure we have the right number of empty words
                self.seed_words = vec![String::new(); word_count];
            }
        }
    }

    // Display recovery seed words in a grid
    fn show_seed_words(&self, ui: &mut Ui) {
        // Calculate words per row for display
        let words_per_row = if self.use_24_words { 6 } else { 4 };
        let rows = self.seed_words.len() / words_per_row;
        
        // Create a frame for the seed words
        Frame::group(ui.style())
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Your Recovery Seed Phrase");
                    ui.label("Write down these words in order and keep them safe. They are the only way to recover your wallet if you lose access.");
                    ui.add_space(20.0);
                    
                    // Create a grid for the seed words
                    Grid::new("seed_words_grid")
                        .num_columns(words_per_row)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            for row in 0..rows {
                                for col in 0..words_per_row {
                                    let index = row * words_per_row + col;
                                    let word_number = index + 1; // 1-indexed for display
                                    ui.label(format!("{}. {}", word_number, self.seed_words[index]));
                                }
                                ui.end_row();
                            }
                        });
                });
            });
    }

    // Get the seed phrase
    pub fn get_seed_phrase(&self) -> String {
        self.seed_words.join(" ")
    }

    pub fn render(&mut self, ctx: &Context) -> Option<Screen> {
        let mut next_screen = None;

        // Set font styles
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
                    ui.heading("Create New Wallet");
                    ui.add_space(20.0);
                    
                    // Set a fixed width for the form
                    let form_width = 600.0;
                    
                    // Create a frame for the form with fixed width
                    Frame::group(ui.style())
                        .inner_margin(20.0)
                        .outer_margin(10.0)
                        .show(ui, |ui| {
                            // Set the maximum width for the content
                            ui.set_max_width(form_width);
                            
                            // Radio buttons to select 12 or 24 words
                            ui.vertical_centered(|ui| {
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
                                    
                                    // Regenerate seed if word count changes
                                    if changed {
                                        self.generate_seed_words();
                                    }
                                });
                            });
                            
                            ui.add_space(20.0);
                            
                            // Display seed words
                            self.show_seed_words(ui);
                            
                            ui.add_space(20.0);
                            
                            // Regenerate seed button
                            ui.vertical_centered(|ui| {
                                let button_size = Vec2::new(200.0, 40.0);
                                if ui.add_sized(button_size, egui::Button::new("Regenerate Seed")).clicked() {
                                    self.generate_seed_words();
                                }
                            });
                            
                            ui.add_space(20.0);
                            
                            // Password fields for encryption
                            ui.vertical_centered(|ui| {
                                ui.label("Enter password to encrypt your new wallet:");
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
                            
                            // Bottom buttons inside the form
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
                                
                                if ui.add_sized(button_size, egui::Button::new("Create Encrypted Wallet")).clicked() {
                                    // Validate passwords
                                    if self.password.is_empty() {
                                        self.status_message = "Error: Password cannot be empty".to_string();
                                    } else if self.password != self.confirm_password {
                                        self.status_message = "Error: Passwords do not match".to_string();
                                    } else {
                                        // Get seed phrase
                                        let seed_phrase = self.get_seed_phrase();
                                        
                                        // Save wallet using the Wallet module
                                        match Wallet::save_wallet(&seed_phrase, &self.password) {
                                            Ok(_) => {
                                                self.status_message = "Wallet saved successfully!".to_string();
                                                // Navigate to main screen after successful wallet creation
                                                next_screen = Some(Screen::MainScreen);
                                            }
                                            Err(e) => {
                                                self.status_message = format!("Error: {}", e);
                                            }
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