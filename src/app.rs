use crate::gfx;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

pub struct App {
    window: Option<Window>,
    window_attribs: WindowAttributes,
    renderer: Option<gfx::Renderer>,
}

impl App {
    pub fn new(window_attribs: WindowAttributes) -> Self {
        Self {
            window: None,
            window_attribs,
            renderer: None,
        }
    }
    pub fn handle_input(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, key: KeyCode) {
        if key == KeyCode::Escape {
            event_loop.exit();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(self.window_attribs.clone())
            .unwrap();

        let renderer = gfx::Renderer::new(&window).unwrap();

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
        if window_id == window.id() {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => {
                    window.request_redraw();
                    self.renderer.as_mut().unwrap().draw().unwrap();
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(key),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => self.handle_input(event_loop, key),
                _ => {}
            }
        }
    }
}
