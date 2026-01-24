mod config;
mod instance;
mod keylogger;
mod manager;
mod projector;
mod utils;
mod window;
mod wnd_class;

fn main() {
    window::spawn();
}
