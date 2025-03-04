use egui::{CentralPanel, Context, FontId, TextStyle, Vec2, TopBottomPanel, Ui, RichText, Color32, Window, Rect, Pos2, Stroke, Rounding, SidePanel};
use super::Screen;
use std::time::{Duration, Instant};
use crate::core::rpc::RpcClient;
use crate::core::img2hex::{self, image_to_hex};
use crate::core::wallet::Wallet;
use solana_sdk::signature::Keypair;
use super::panels::{
    inscription::InscriptionPanel,
    panel_a::PanelA,
    panel_b::PanelB,
};

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
    // Wallet instance
    wallet: Wallet,
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
            last_balance_update: None,
            rpc_client: RpcClient::default_testnet(),
            selected_menu: MenuItem::Inscription,  // Default to Inscription
            show_image_dialog: false,
            image_hex: None,
            image_binary: None,
            inscription_panel: InscriptionPanel::new(),
            panel_a: PanelA::new(),
            panel_b: PanelB::new(),
        }
    }
    
    // Query balance from RPC
    fn query_balance(&self) -> Result<f64, String> {
        self.rpc_client.get_balance(&self.wallet.address)
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