use eframe::egui;
use egui_hooks::UseHookExt as _;

fn main() {
    eframe::run_native("example", Default::default(), Box::new(|_| Box::new(MyApp))).unwrap();
}

struct MyApp;

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let count = ui.use_state(|| 0usize, ());
            ui.use_effect(|| println!("Count changed to {}", *count), count.clone());
            ui.label(format!("Count: {}", count));
            if ui.button("Increment").clicked() {
                count.set_next(*count + 1);
            }
        });
    }
}
