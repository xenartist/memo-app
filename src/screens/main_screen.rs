use egui::{CentralPanel, Context, FontId, TextStyle, Vec2, TopBottomPanel, Ui, RichText, Color32};
use super::Screen;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use ed25519_dalek::SigningKey;
use bs58;

type HmacSha512 = Hmac<Sha512>;

pub struct MainScreen {
    // Wallet public key (Solana address)
    wallet_address: String,
    // Seed phrase (stored temporarily)
    seed_phrase: String,
}

impl MainScreen {
    pub fn new(seed_phrase: &str) -> Self {
        let mut screen = Self {
            wallet_address: String::new(),
            seed_phrase: seed_phrase.to_string(),
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
    
    // Show wallet address in the top panel
    fn show_wallet_address(&self, ui: &mut Ui) {
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
            if ui.button(RichText::new("📋 Copy Full Address").size(20.0)).clicked() {
                ui.ctx().copy_text(self.wallet_address.clone());
            }
        });
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

        // Main content
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Memo App Main Screen");
                ui.add_space(20.0);
                
                // Main content will go here
                ui.label("Welcome to your wallet!");
                
                ui.add_space(20.0);
                
                // Logout button
                let button_size = Vec2::new(150.0, 40.0);
                if ui.add_sized(button_size, egui::Button::new("Logout")).clicked() {
                    next_screen = Some(Screen::Login);
                }
            });
        });

        next_screen
    }
} 