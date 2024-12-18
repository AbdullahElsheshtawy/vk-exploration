use app::App;
use winit::{event_loop::EventLoop, window::WindowAttributes};

mod app;
mod gfx;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new()?;

    let mut app = App::new(WindowAttributes::default(), true);
    event_loop.run_app(&mut app)?;

    Ok(())
}
