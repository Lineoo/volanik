#![windows_subsystem = "windows"]

mod clouds;
mod render;
mod window;

fn main() {
    env_logger::init();

    log::info!("This is Volanik. Welcome!");

    let window = window::MainEventLoop::default();
    window.run();
}

struct Volanik {
    render: render::Render,
    clouds: clouds::Clouds,
}

impl Volanik {
    fn render(&self) {
        self.render.render(|rpass| {
            self.clouds.render(rpass);
        });
    }
}
