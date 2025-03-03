use egui::Ui;

pub struct PanelA;

impl PanelA {
    pub fn new() -> Self {
        Self
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Panel A");
            ui.add_space(20.0);
            ui.label("This is Panel A content");
        });
    }
} 