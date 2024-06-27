mod app;

use app::App;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("SOL Tools", native_options, Box::new(|_| Box::new(App::default())))
        .unwrap();
}
