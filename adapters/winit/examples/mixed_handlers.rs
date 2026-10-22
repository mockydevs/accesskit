mod util;

use accesskit::{ActivationHandler, TreeUpdate};
use accesskit_winit::{Adapter, WindowEvent as AccessKitWindowEvent};
use example_common::{Modifiers, Renderer, UiState, WINDOW_TITLE};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

struct TearoffActivationHandler {
    ui: Arc<Mutex<UiState>>,
}

impl ActivationHandler for TearoffActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(self.ui.lock().unwrap().build_tree_update())
    }
}

struct WindowState {
    // Declared first so that the renderer is dropped before the window.
    renderer: Renderer<Arc<dyn Window>>,
    window: Arc<dyn Window>,
    adapter: Adapter,
    ui: Arc<Mutex<UiState>>,
    modifiers: Modifiers,
}

impl WindowState {
    fn new(window: Arc<dyn Window>, adapter: Adapter, ui: Arc<Mutex<UiState>>) -> Self {
        Self {
            renderer: Renderer::new(Arc::clone(&window)),
            window,
            adapter,
            ui,
            modifiers: Modifiers::default(),
        }
    }

    fn update_accessibility_tree(&mut self) {
        let mut ui = self.ui.lock().unwrap();
        self.adapter.update_if_active(|| ui.build_tree_update());
    }
}

struct Application {
    events: util::EventQueue,
    window: Option<WindowState>,
}

impl Application {
    fn new(event_loop_proxy: EventLoopProxy) -> Self {
        Self {
            events: util::EventQueue::new(event_loop_proxy),
            window: None,
        }
    }

    fn create_window(&mut self, event_loop: &dyn ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window_attributes = winit::window::WindowAttributes::default()
            .with_title(WINDOW_TITLE)
            .with_visible(false);

        let window: Arc<dyn Window> = Arc::from(event_loop.create_window(window_attributes)?);
        let mut ui = UiState::new();
        util::set_viewport(&mut ui, window.as_ref());
        let ui = Arc::new(Mutex::new(ui));
        let activation_handler = TearoffActivationHandler {
            ui: Arc::clone(&ui),
        };
        let adapter = Adapter::with_mixed_handlers(
            event_loop,
            window.as_ref(),
            activation_handler,
            self.events.callback(),
        );
        window.set_visible(true);

        self.window = Some(WindowState::new(window, adapter, ui));
        Ok(())
    }
}

impl ApplicationHandler for Application {
    fn window_event(&mut self, _: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(window) = &mut self.window else {
            return;
        };

        window.adapter.process_event(window.window.as_ref(), &event);
        match event {
            WindowEvent::CloseRequested => {
                self.window = None;
            }
            WindowEvent::SurfaceResized(_) => {
                util::set_viewport(&mut window.ui.lock().unwrap(), window.window.as_ref());
                window.update_accessibility_tree();
                window.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let size = window.window.surface_size();
                window.renderer.draw(size.width, size.height);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                window.modifiers = util::modifiers(&modifiers);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let events = util::key_events(&event, window.modifiers);
                if !events.is_empty() {
                    for event in events {
                        window.ui.lock().unwrap().handle_key(event);
                    }
                    window.update_accessibility_tree();
                    window.window.request_redraw();
                }
            }
            _ => (),
        }
    }

    fn proxy_wake_up(&mut self, _: &dyn ActiveEventLoop) {
        for user_event in self.events.take() {
            let Some(window) = &mut self.window else {
                return;
            };

            if user_event.window_id != window.window.id() {
                continue;
            }
            match user_event.window_event {
                // The tearoff activation handler takes care of this.
                AccessKitWindowEvent::InitialTreeRequested => unreachable!(),
                AccessKitWindowEvent::ActionRequested(request) => {
                    window.ui.lock().unwrap().do_action(&request);
                    window.update_accessibility_tree();
                    window.window.request_redraw();
                }
                AccessKitWindowEvent::AccessibilityDeactivated => {
                    window.ui.lock().unwrap().deactivated();
                }
            }
        }
    }

    fn destroy_surfaces(&mut self, _: &dyn ActiveEventLoop) {
        self.window = None;
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_none() {
            self.create_window(event_loop)
                .expect("failed to create initial window");
        }
        if let Some(window) = self.window.as_ref() {
            window.window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        let Some(window) = &mut self.window else {
            event_loop.exit();
            return;
        };

        let processed = util::process_due_messages(event_loop, &mut window.ui.lock().unwrap());
        if processed {
            window.update_accessibility_tree();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    example_common::print_instructions();

    let event_loop = EventLoop::builder().build()?;
    let state = Application::new(event_loop.create_proxy());
    event_loop.run_app(state).map_err(Into::into)
}
