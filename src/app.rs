use crate::gfx;
use anyhow::Context;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::PhysicalKey,
    window::{Window, WindowAttributes},
};

pub struct App {
    window: Option<Window>,
    window_attribs: WindowAttributes,
    renderer: Option<gfx::Renderer>,
    debug: bool,
}

impl App {
    pub fn new(window_attribs: WindowAttributes, debug: bool) -> Self {
        Self {
            window: None,
            window_attribs,
            renderer: None,
            debug,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(self.window_attribs.clone())
            .map_err(|err| log::error!("Window could not be created: {}", err))
            .unwrap();

        let renderer = gfx::Renderer::new(self.debug, &window)
            .map_err(|err| log::error!("Renderer could not be initialized: {}", err))
            .unwrap();
        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let window = self.window.as_ref().unwrap();
        let renderer = self.renderer.as_ref().unwrap();

        if window_id == window.id() {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => window.request_redraw(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(key),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    use winit::keyboard::KeyCode;
                    match key {
                        KeyCode::Escape => event_loop.exit(),

                        key_event => log::trace!("Keyboard event not handled: {:?}", key_event),
                    }
                }
                not_handled_event => log::trace!("Event not handled: {:?}", not_handled_event),
            }
        }
    }
}
