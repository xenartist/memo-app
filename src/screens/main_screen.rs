use egui::{CentralPanel, Context, FontId, TextStyle, Vec2, TopBottomPanel, Ui, RichText, Color32, Window, Rect, Pos2, Stroke, Rounding, SidePanel};
use super::Screen;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use ed25519_dalek::SigningKey;
use bs58;
use std::time::{Duration, Instant};
use crate::core::rpc::RpcClient;
use crate::core::img2hex::{self, image_to_hex};
use image::DynamicImage;
use super::panels::{
    inscription::InscriptionPanel,
    panel_a::PanelA,
    panel_b::PanelB,
};

type HmacSha512 = Hmac<Sha512>;

// Menu items
#[derive(Debug, Clone, Copy, PartialEq)]
enum MenuItem {
    Inscription,
    PanelA,
    PanelB,
}

impl MenuItem {
    fn label(&self) -> &'static str {
        match self {
            MenuItem::Inscription => "Inscription",
            MenuItem::PanelA => "Panel A",
            MenuItem::PanelB => "Panel B",
        }
    }
}

pub struct MainScreen {
    // Wallet public key (Solana address)
    wallet_address: String,
    // Seed phrase (stored temporarily)
    seed_phrase: String,
    // Balance in SOL
    balance: f64,
    // Last balance update time
    last_balance_update: Option<Instant>,
    rpc_client: RpcClient,
    // Current selected menu item
    selected_menu: MenuItem,
    // Image dialog state
    show_image_dialog: bool,
    // Imported image hex string
    image_hex: Option<String>,
    // Binary representation for display
    image_binary: Option<String>,
    // Panel instances
    inscription_panel: InscriptionPanel,
    panel_a: PanelA,
    panel_b: PanelB,
}

impl MainScreen {
    pub fn new(seed_phrase: &str) -> Self {
        let mut screen = Self {
            wallet_address: String::new(),
            seed_phrase: seed_phrase.to_string(),
            balance: 0.0,
            last_balance_update: None,
            rpc_client: RpcClient::default_testnet(),
            selected_menu: MenuItem::Inscription,  // Default to Inscription
            show_image_dialog: false,
            image_hex: None,
            image_binary: None,
            inscription_panel: InscriptionPanel::new(),
            panel_a: PanelA::new(),
            panel_b: PanelB::new(),
        };
        
        // Generate wallet address from seed
        screen.generate_wallet_address();
        
        screen
    }
    
    // Generate Solana wallet address from seed phrase
    fn generate_wallet_address(&mut self) {
        if self.seed_phrase.is_empty() {
            self.wallet_address = "No wallet loaded".to_string();
            return;
        }
        
        // Derive the private key using BIP44 for Solana (m/44'/501'/0'/0')
        let private_key = match self.derive_private_key() {
            Ok(key) => key,
            Err(e) => {
                self.wallet_address = format!("Error generating address: {}", e);
                return;
            }
        };
        
        // Convert private key to public key
        let signing_key = SigningKey::from_bytes(&private_key);
        let public_key = signing_key.verifying_key();
        
        // Convert public key to Solana address (base58 encoding of public key bytes)
        let address = bs58::encode(public_key.as_bytes()).into_string();
        self.wallet_address = address;
    }
    
    // Derive private key using BIP44 for Solana (m/44'/501'/0'/0')
    fn derive_private_key(&self) -> Result<[u8; 32], String> {
        // Convert seed phrase to seed bytes
        let seed = self.seed_to_bytes()?;
        
        // BIP44 path for Solana: m/44'/501'/0'/0'
        let path = "m/44'/501'/0'/0'";
        
        // Derive master key
        let (master_key, chain_code) = self.derive_master_key(&seed)?;
        
        // Derive child keys according to path
        let mut key = master_key;
        let mut code = chain_code;
        
        // Parse path and derive each level
        let path_components: Vec<&str> = path.split('/').collect();
        for &component in path_components.iter().skip(1) { // Skip 'm'
            let hardened = component.ends_with('\'');
            let index_str = if hardened {
                &component[0..component.len()-1]
            } else {
                component
            };
            
            let mut index = index_str.parse::<u32>().map_err(|_| "Invalid path component".to_string())?;
            if hardened {
                index += 0x80000000; // Hardened key
            }
            
            // Derive child key
            let (child_key, child_code) = self.derive_child_key(&key, &code, index)?;
            key = child_key;
            code = child_code;
        }
        
        // Create 32-byte array for the key
        let mut result = [0u8; 32];
        result.copy_from_slice(&key[0..32]);
        
        Ok(result)
    }
    
    // Convert seed phrase to seed bytes
    fn seed_to_bytes(&self) -> Result<Vec<u8>, String> {
        // Use BIP39 to convert mnemonic to seed
        let mnemonic = bip39::Mnemonic::parse_normalized(&self.seed_phrase)
            .map_err(|e| format!("Invalid mnemonic: {}", e))?;
        
        // Generate seed with empty passphrase
        let seed = mnemonic.to_seed("");
        
        Ok(seed.to_vec())
    }
    
    // Derive master key from seed
    fn derive_master_key(&self, seed: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        // HMAC-SHA512 with key "ed25519 seed"
        let mut mac = HmacSha512::new_from_slice(b"ed25519 seed")
            .map_err(|_| "Failed to create HMAC".to_string())?;
        
        mac.update(seed);
        let result = mac.finalize().into_bytes();
        
        // Split result into key and chain code
        let key = result[0..32].to_vec();
        let chain_code = result[32..64].to_vec();
        
        Ok((key, chain_code))
    }
    
    // Derive child key
    fn derive_child_key(&self, key: &[u8], chain_code: &[u8], index: u32) -> Result<(Vec<u8>, Vec<u8>), String> {
        let mut mac = HmacSha512::new_from_slice(chain_code)
            .map_err(|_| "Failed to create HMAC".to_string())?;
        
        // For hardened keys, use 0x00 || key || index
        // For normal keys, use public_key || index
        if index >= 0x80000000 {
            mac.update(&[0x00]);
            mac.update(key);
        } else {
            // For normal derivation, we would use the public key, but Solana uses hardened derivation
            // This branch shouldn't be reached for Solana wallets
            return Err("Non-hardened derivation not supported for Solana".to_string());
        }
        
        // Append index in big-endian
        mac.update(&[(index >> 24) as u8, (index >> 16) as u8, (index >> 8) as u8, index as u8]);
        
        let result = mac.finalize().into_bytes();
        
        // Split result into key and chain code
        let derived_key = result[0..32].to_vec();
        let derived_chain_code = result[32..64].to_vec();
        
        Ok((derived_key, derived_chain_code))
    }
    
    // Format wallet address with mask
    fn format_masked_address(&self) -> String {
        if self.wallet_address.len() < 8 {
            return self.wallet_address.clone();
        }
        
        let prefix = &self.wallet_address[..4];
        let suffix = &self.wallet_address[self.wallet_address.len() - 4..];
        format!("{}****{}", prefix, suffix)
    }
    
    // Query balance from RPC
    fn query_balance(&self) -> Result<f64, String> {
        self.rpc_client.get_balance(&self.wallet_address)
    }

    // Update balance if needed
    fn update_balance(&mut self) {
        let should_update = match self.last_balance_update {
            None => true,
            Some(last_update) => last_update.elapsed() > Duration::from_secs(30), // Update every 30 seconds
        };

        if should_update {
            match self.query_balance() {
                Ok(balance) => {
                    self.balance = balance;
                    self.last_balance_update = Some(Instant::now());
                }
                Err(_) => {
                    // If there's an error, we'll keep the old balance
                    if self.last_balance_update.is_none() {
                        self.balance = 0.0;
                        self.last_balance_update = Some(Instant::now());
                    }
                }
            }
        }
    }
    
    // Show wallet address in the top panel
    fn show_wallet_address(&mut self, ui: &mut Ui) {
        // Update balance before displaying
        self.update_balance();

        ui.horizontal(|ui| {
            ui.label(RichText::new("Wallet Address: ").size(20.0));
            
            // Display masked address with different color
            ui.label(
                RichText::new(&self.format_masked_address())
                    .color(Color32::LIGHT_BLUE)
                    .monospace()
                    .size(20.0)
            );
            
            // Add copy button (copies the full address)
            if ui.button(RichText::new("📋 Copy").size(20.0)).clicked() {
                ui.ctx().copy_text(self.wallet_address.clone());
            }

            ui.add_space(10.0);
            
            // Display balance
            ui.label(
                RichText::new(format!("Balance: {:.9} SOL", self.balance))
                    .color(Color32::LIGHT_GREEN)
                    .monospace()
                    .size(20.0)
            );
        });
    }

    // Show image import dialog
    fn show_image_dialog(&mut self, ui: &mut Ui) {
        Window::new("New Inscription")
            .default_size(Vec2::new(600.0, 700.0))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Import and Convert Image");
                    ui.add_space(20.0);
                    
                    // Import button
                    if ui.button("Choose Image").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Image", &["png", "jpg", "jpeg", "gif", "bmp"])
                            .pick_file() 
                        {
                            // Load and convert image
                            if let Ok(img) = image::open(&path) {
                                let hex = image_to_hex(&img);
                                self.image_binary = Some(img2hex::hex_to_binary(&hex));
                                self.image_hex = Some(hex);
                            }
                        }
                    }
                    
                    ui.add_space(20.0);
                    
                    // Display pixel art if available
                    if let Some(binary) = &self.image_binary {
                        // Calculate pixel size based on available width
                        let available_width = ui.available_width().min(500.0);
                        let pixel_size = (available_width / 48.0).floor();
                        let total_size = pixel_size * 48.0;
                        
                        // Create a frame for the pixel art
                        let (response, painter) = ui.allocate_painter(
                            Vec2::new(total_size, total_size),
                            egui::Sense::hover()
                        );
                        
                        let rect = response.rect;
                        
                        // Draw background
                        painter.rect_filled(rect, Rounding::default(), Color32::WHITE);
                        
                        // Draw pixels
                        for (i, bit) in binary.chars().enumerate() {
                            if bit == '1' {
                                let x = (i % 48) as f32 * pixel_size;
                                let y = (i / 48) as f32 * pixel_size;
                                
                                painter.rect_filled(
                                    Rect::from_min_size(
                                        Pos2::new(rect.min.x + x, rect.min.y + y),
                                        Vec2::new(pixel_size, pixel_size)
                                    ),
                                    Rounding::default(),
                                    Color32::BLACK
                                );
                            }
                        }
                        
                        // Draw grid
                        for i in 0..=48 {
                            let x = rect.min.x + i as f32 * pixel_size;
                            let y = rect.min.y + i as f32 * pixel_size;
                            
                            painter.line_segment(
                                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                                Stroke::new(0.5, Color32::GRAY)
                            );
                            painter.line_segment(
                                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                                Stroke::new(0.5, Color32::GRAY)
                            );
                        }
                        
                        ui.add_space(20.0);
                        
                        // Display hex string
                        if let Some(hex) = &self.image_hex {
                            ui.label("Hex representation:");
                            ui.add_space(5.0);
                            ui.label(RichText::new(hex).monospace());
                        }
                    }
                });
            });
    }

    // Show left menu panel
    fn show_menu_panel(&mut self, ui: &mut Ui) {
        for menu_item in [MenuItem::Inscription, MenuItem::PanelA, MenuItem::PanelB].iter() {
            let is_selected = &self.selected_menu == menu_item;
            let text = RichText::new(menu_item.label())
                .size(22.0)
                .strong()
                .color(if is_selected { Color32::WHITE } else { Color32::LIGHT_GRAY });
            
            if ui.add(egui::SelectableLabel::new(is_selected, text)).clicked() {
                self.selected_menu = *menu_item;
            }
            ui.add_space(10.0);
        }
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

        // Top panel for wallet address
        TopBottomPanel::top("wallet_address_panel").show(ctx, |ui| {
            ui.add_space(10.0);
            self.show_wallet_address(ui);
            ui.add_space(10.0);
            ui.separator();
        });

        // Left menu panel
        SidePanel::left("menu_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.add_space(20.0);
                self.show_menu_panel(ui);
                
                // Add logout button at the bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    if ui.button(RichText::new("Logout").size(22.0)).clicked() {
                        next_screen = Some(Screen::Login);
                    }
                    ui.add_space(20.0);
                });
            });

        // Main content area
        CentralPanel::default().show(ctx, |ui| {
            match self.selected_menu {
                MenuItem::Inscription => self.inscription_panel.show(ui),
                MenuItem::PanelA => self.panel_a.show(ui),
                MenuItem::PanelB => self.panel_b.show(ui),
            }
        });

        next_screen
    }
} 