use egui::{CentralPanel, Context, FontId, TextStyle, Grid, RadioButton, Ui, ScrollArea, Frame, Vec2};
use super::Screen;
use bip39::Mnemonic;
use rand::{rngs::OsRng, RngCore};

pub struct NewWalletScreen {
    // Store recovery seed words
    seed_words: Vec<String>,
    // Whether to use 24 words (otherwise use 12)
    use_24_words: bool,
}

impl Default for NewWalletScreen {
    fn default() -> Self {
        let mut screen = Self {
            seed_words: Vec::new(),
            use_24_words: false,
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
        // Determine entropy size based on word count
        // 16 bytes (128 bits) for 12 words, 32 bytes (256 bits) for 24 words
        let entropy_size = if self.use_24_words { 32 } else { 16 };
        
        // Generate random entropy
        let mut entropy = vec![0u8; entropy_size];
        OsRng.fill_bytes(&mut entropy);
        
        // Create mnemonic from entropy
        let mnemonic = Mnemonic::from_entropy(&entropy).expect("Failed to generate mnemonic");
        
        // Get the phrase as a string and split into words
        let phrase = mnemonic.to_string();
        self.seed_words = phrase.split_whitespace().map(String::from).collect();
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

    pub fn render(&mut self, ctx: &Context) -> Option<Screen> {
        let mut next_screen = None;

        // Set font styles
        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(20.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(30.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(18.0, egui::FontFamily::Proportional)
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
                            
                            // Bottom buttons inside the form
                            ui.horizontal(|ui| {
                                // Calculate button sizes and spacing
                                let button_size = Vec2::new(150.0, 40.0);
                                let available_width = ui.available_width();
                                let spacing = (available_width - 2.0 * button_size.x) / 3.0;
                                
                                ui.add_space(spacing);
                                
                                if ui.add_sized(button_size, egui::Button::new("Back")).clicked() {
                                    next_screen = Some(Screen::Login);
                                }
                                
                                ui.add_space(spacing);
                                
                                if ui.add_sized(button_size, egui::Button::new("Regenerate Seed")).clicked() {
                                    self.generate_seed_words();
                                }
                            });
                        });
                    
                    ui.add_space(20.0);
                });
            });
        });

        next_screen
    }
} 