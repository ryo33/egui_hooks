use eframe::egui;
use egui_hooks::UseHookExt as _;

fn main() {
    eframe::run_native(
        "example",
        Default::default(),
        Box::new(|_| Ok(Box::new(MyApp))),
    )
    .unwrap();
}

struct MyApp;

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let count = use_counter(ui, 0);
            let count2 = use_counter(ui, 0);
            ui.label(format!("Count: {}", count));
            ui.label(format!("Count2: {}", count2));
        });
    }
}

fn use_counter(ui: &mut egui::Ui, initial: usize) -> usize {
    let count = ui.use_state(|| initial, ());
    if ui.button("Increment").clicked() {
        count.set_next(*count + 1);
    }
    *count
}
