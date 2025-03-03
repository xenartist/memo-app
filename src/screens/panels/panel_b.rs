use egui::Ui;

pub struct PanelB;

impl PanelB {
    pub fn new() -> Self {
        Self
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Panel B");
            ui.add_space(20.0);
            ui.label("This is Panel B content");
        });
    }
} 