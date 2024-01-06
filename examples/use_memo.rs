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
            let memo = ui.use_memo(
                || {
                    println!("Calculating memoized value");
                    count.pow(2)
                },
                count.clone(),
            );
            ui.label(format!("Count: {}", count));
            ui.label(format!("Memo: {}", memo));
            if ui.button("Increment").clicked() {
                count.set_next(*count + 1);
            }
        });
    }
}
