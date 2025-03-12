use egui::{CentralPanel, Context, FontId, TextStyle, TopBottomPanel, Ui, RichText, Color32, SidePanel};
use super::Screen;
use std::time::{Duration, Instant};
use crate::core::rpc::RpcClient;
use crate::core::wallet::Wallet;
use crate::core::memo;
use super::password_dialog::PasswordDialog;
use crate::screens::panels::inscription::InscriptionPanel;
use crate::screens::panels::panel_a::PanelA;
use crate::screens::panels::panel_b::PanelB;
use crate::screens::panels::settings_panel::SettingsPanel;
use std::sync::{Arc, Mutex};

// Menu items
#[derive(Debug, Clone, Copy, PartialEq)]
enum MenuItem {
    Inscription,
    PanelA,
    PanelB,
    Settings,
}

impl MenuItem {
    fn label(&self) -> &'static str {
        match self {
            MenuItem::Inscription => "Inscription",
            MenuItem::PanelA => "Panel A",
            MenuItem::PanelB => "Panel B",
            MenuItem::Settings => "Settings",
        }
    }
}

pub struct MainScreen {
    // Wallet instance
    wallet: Wallet,
    // Balance in SOL
    balance: f64,
    // Token balance
    token_balance: f64,
    // Last balance update time
    last_balance_update: Option<Instant>,
    // Last token balance update time
    last_token_balance_update: Option<Instant>,
    rpc_client: RpcClient,
    // Current selected menu item
    selected_menu: MenuItem,
    // Status message for inscription
    inscription_status: Option<String>,
    // Password dialog for secure operations
    password_dialog: PasswordDialog,
    // Pending hex data for inscription (waiting for password)
    pending_inscription_hex: Option<String>,
    // Shared status for inscription process
    inscription_status_arc: Option<Arc<Mutex<String>>>,
    // Panel instances
    inscription_panel: InscriptionPanel,
    panel_a: PanelA,
    panel_b: PanelB,
    settings_panel: SettingsPanel,
}

impl MainScreen {
    pub fn new(seed_phrase: &str) -> Self {
        // Create wallet from seed phrase
        let wallet = match Wallet::new(seed_phrase) {
            Ok(wallet) => wallet,
            Err(_) => {
                // Create a wallet with empty address in case of error
                Wallet::new_with_address("Error loading wallet")
            }
        };
        
        Self {
            wallet,
            balance: 0.0,
            token_balance: 0.0,
            last_balance_update: None,
            last_token_balance_update: None,
            rpc_client: RpcClient::default_testnet(),
            selected_menu: MenuItem::Inscription,  // Default to Inscription
            inscription_status: None,
            password_dialog: PasswordDialog::new(),
            pending_inscription_hex: None,
            inscription_status_arc: None,
            inscription_panel: InscriptionPanel::new(),
            panel_a: PanelA::new(),
            panel_b: PanelB::new(),
            settings_panel: SettingsPanel::new(),
        }
    }
    
    // Create a new MainScreen with just a wallet address
    pub fn new_with_address(address: &str) -> Self {
        // Create wallet with address
        let wallet = Wallet::new_with_address(address);
        
        Self {
            wallet,
            balance: 0.0,
            token_balance: 0.0,
            last_balance_update: None,
            last_token_balance_update: None,
            rpc_client: RpcClient::default_testnet(),
            selected_menu: MenuItem::Inscription,  // Default to Inscription
            inscription_status: None,
            password_dialog: PasswordDialog::new(),
            pending_inscription_hex: None,
            inscription_status_arc: None,
            inscription_panel: InscriptionPanel::new(),
            panel_a: PanelA::new(),
            panel_b: PanelB::new(),
            settings_panel: SettingsPanel::new(),
        }
    }
    
    // Get the wallet address
    pub fn get_wallet_address(&self) -> String {
        self.wallet.address.clone()
    }
    
    // Query balance from RPC
    fn query_balance(&self) -> Result<f64, String> {
        self.rpc_client.get_balance(&self.wallet.address)
    }
    
    // Query token balance
    fn query_token_balance(&self) -> Result<f64, String> {
        memo::get_token_balance_for_address(&self.wallet.address)
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
    
    // Update token balance if needed
    fn update_token_balance(&mut self) {
        let should_update = match self.last_token_balance_update {
            None => true,
            Some(last_update) => last_update.elapsed() > Duration::from_secs(30), // Update every 30 seconds
        };

        if should_update {
            match self.query_token_balance() {
                Ok(balance) => {
                    self.token_balance = balance;
                    self.last_token_balance_update = Some(Instant::now());
                }
                Err(_) => {
                    // If there's an error, we'll keep the old balance
                    if self.last_token_balance_update.is_none() {
                        self.token_balance = 0.0;
                        self.last_token_balance_update = Some(Instant::now());
                    }
                }
            }
        }
    }
    
    // Show wallet address in the top panel
    fn show_wallet_address(&mut self, ui: &mut Ui) {
        // Update balance before displaying
        self.update_balance();
        self.update_token_balance();

        ui.horizontal(|ui| {
            ui.label(RichText::new("Wallet Address: ").size(20.0));
            
            // Display masked address with different color
            ui.label(
                RichText::new(&self.wallet.format_masked_address())
                    .color(Color32::LIGHT_BLUE)
                    .monospace()
                    .size(20.0)
            );
            
            // Add copy button (copies the full address)
            if ui.button(RichText::new("📋 Copy").size(20.0)).clicked() {
                ui.ctx().copy_text(self.wallet.address.clone());
            }

            ui.add_space(10.0);
            
            // Display SOL balance
            ui.label(
                RichText::new(format!("Balance: {:.9} SOL", self.balance))
                    .color(Color32::LIGHT_GREEN)
                    .monospace()
                    .size(20.0)
            );
            
            ui.add_space(10.0);
            
            // Display token balance
            ui.label(
                RichText::new(format!("Tokens: {:.2}", self.token_balance))
                    .color(Color32::GOLD)
                    .monospace()
                    .size(20.0)
            );
        });
    }

    // Show left menu panel
    fn show_menu_panel(&mut self, ui: &mut Ui) {
        for menu_item in [MenuItem::Inscription, MenuItem::PanelA, MenuItem::PanelB, MenuItem::Settings].iter() {
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

        // Set up the callback for the inscription panel
        let self_ptr = self as *mut MainScreen;
        self.inscription_panel.set_inscription_callback(move |hex_data| {
            // Safety: This is safe because the callback is executed in the same thread
            // where the UI is running, not in a separate thread
            unsafe {
                // Request password through the password dialog
                (*self_ptr).password_dialog.request_password(move |password| {
                    (*self_ptr).process_inscription(format!("{}:{}", password, hex_data));
                });
            }
        });

        // Check if we have a status update to apply
        let mut should_clear_status = false;
        if let Some(status_arc) = &self.inscription_status_arc {
            if let Ok(status) = status_arc.lock() {
                let status_str = status.clone();
                self.inscription_panel.set_inscription_status(status_str.clone());
                self.inscription_status = Some(status_str.clone());
                
                // Check if this is a final status
                if status_str.starts_with("Success") || status_str.starts_with("Error") {
                    should_clear_status = true;
                }
            }
        }
        
        // Clear status if needed
        if should_clear_status {
            self.inscription_status = None;
            self.inscription_status_arc = None;
            self.pending_inscription_hex = None;
        }

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
                MenuItem::Settings => self.settings_panel.show(ui),
            }
        });

        // Show password dialog if needed
        self.password_dialog.show(ctx);

        // Request a repaint to keep checking status
        if self.inscription_status_arc.is_some() {
            ctx.request_repaint();
        }

        next_screen
    }

    // Process inscription with password and hex data
    fn process_inscription(&mut self, password_and_hex: String) {
        // Split the input string into password and hex data
        let parts: Vec<&str> = password_and_hex.split(':').collect();
        if parts.len() != 2 {
            self.inscription_panel.set_inscription_status(
                "Error: Invalid input format".to_string()
            );
            return;
        }
        
        let password = parts[0].to_string();
        let hex_data = parts[1].to_string();
        
        // Clone necessary data for the async task
        let wallet = self.wallet.clone();
        let hex_data_clone = hex_data.clone();
        let password_clone = password.clone();
        
        // Create a shared status variable
        let status = Arc::new(Mutex::new(String::from("Creating inscription...")));
        let status_for_thread = Arc::clone(&status);
        
        // Set initial status
        self.inscription_status = Some("Creating inscription...".to_string());
        self.inscription_panel.set_inscription_status("Creating inscription...".to_string());
        
        // Store the status Arc for checking in render
        self.pending_inscription_hex = Some(hex_data);
        self.inscription_status_arc = Some(Arc::clone(&status));
        
        // Spawn a thread to execute the async task
        std::thread::spawn(move || {
            // Helper function to update status
            let update_status = |msg: &str| {
                if let Ok(mut status) = status_for_thread.lock() {
                    *status = msg.to_string();
                }
            };
            
            // Create a tokio runtime for the async task
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    update_status(&format!("Error: Failed to create runtime: {}", e));
                    return;
                }
            };
            
            // Execute the async task
            let result = rt.block_on(async {
                // Get the keypair using the password
                update_status("Getting keypair...");
                let keypair = match wallet.get_keypair_with_password(&password_clone) {
                    Ok(kp) => kp,
                    Err(e) => {
                        update_status(&format!("Error: Failed to get keypair: {}", e));
                        return Err(format!("Failed to get keypair: {}", e));
                    }
                };
                
                // Create memo client
                update_status("Creating memo client...");
                let memo_client = match memo::create_memo_client(keypair) {
                    Ok(client) => client,
                    Err(e) => {
                        update_status(&format!("Error: Failed to create memo client: {}", e));
                        return Err(format!("Failed to create memo client: {}", e));
                    }
                };
                
                // Mint with memo
                update_status("Minting inscription...");
                match memo_client.mint_with_memo(hex_data_clone).await {
                    Ok(signature) => Ok(signature),
                    Err(e) => {
                        update_status(&format!("Error: Failed to mint inscription: {}", e));
                        Err(format!("Failed to mint inscription: {}", e))
                    }
                }
            });
            
            // Update the final status
            let status_message = match result {
                Ok(signature) => format!("Success! Inscription created with signature: {}", signature),
                Err(e) => format!("Error: {}", e),
            };
            update_status(&status_message);
        });
    }
} 