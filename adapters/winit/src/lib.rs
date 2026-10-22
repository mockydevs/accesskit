// Copyright 2022 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file).

/// ## Compatibility with async runtimes
///
/// The following only applies on Linux/Unix:
///
/// While this crate's API is purely blocking, it internally spawns asynchronous tasks on an executor.
///
/// - If you use tokio, make sure to enable the `tokio` feature of this crate.
/// - If you use another async runtime or if you don't use one at all, the default feature will suit your needs.

#[cfg(all(
    feature = "accesskit_unix",
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    not(feature = "async-io"),
    not(feature = "tokio")
))]
compile_error!("Either \"async-io\" (default) or \"tokio\" feature must be enabled.");

#[cfg(all(
    feature = "accesskit_unix",
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "async-io",
    feature = "tokio"
))]
compile_error!(
    "Both \"async-io\" (default) and \"tokio\" features cannot be enabled at the same time."
);

#[cfg(not(feature = "rwh_06"))]
compile_error!("winit 0.31 requires the rwh_06 feature.");

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, TreeUpdate};
use winit::{
    event::WindowEvent as WinitWindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

#[cfg(feature = "rwh_06")]
#[allow(unused)]
use rwh_06 as raw_window_handle;

mod platform_impl;

#[derive(Debug)]
pub struct Event {
    pub window_id: WindowId,
    pub window_event: WindowEvent,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowEvent {
    InitialTreeRequested,
    ActionRequested(ActionRequest),
    AccessibilityDeactivated,
}

struct WinitActivationHandler<F: Fn(Event) + Send + Sync + 'static> {
    window_id: WindowId,
    callback: std::sync::Arc<F>,
}

impl<F: Fn(Event) + Send + Sync + 'static> ActivationHandler for WinitActivationHandler<F> {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        let event = Event {
            window_id: self.window_id,
            window_event: WindowEvent::InitialTreeRequested,
        };
        (self.callback)(event);
        None
    }
}

struct WinitActionHandler<F: Fn(Event) + Send + Sync + 'static> {
    window_id: WindowId,
    callback: std::sync::Arc<F>,
}

impl<F: Fn(Event) + Send + Sync + 'static> ActionHandler for WinitActionHandler<F> {
    fn do_action(&mut self, request: ActionRequest) {
        let event = Event {
            window_id: self.window_id,
            window_event: WindowEvent::ActionRequested(request),
        };
        (self.callback)(event);
    }
}

struct WinitDeactivationHandler<F: Fn(Event) + Send + Sync + 'static> {
    window_id: WindowId,
    callback: std::sync::Arc<F>,
}

impl<F: Fn(Event) + Send + Sync + 'static> DeactivationHandler for WinitDeactivationHandler<F> {
    fn deactivate_accessibility(&mut self) {
        let event = Event {
            window_id: self.window_id,
            window_event: WindowEvent::AccessibilityDeactivated,
        };
        (self.callback)(event);
    }
}

pub struct Adapter {
    inner: platform_impl::Adapter,
}

impl Adapter {
    /// Creates a new AccessKit adapter for a winit window. This must be done
    /// before the window is shown for the first time. This means that you must
    /// use [`winit::window::WindowAttributes::with_visible`] to make the window
    /// initially invisible, then create the adapter, then show the window.
    ///
    /// This constructor uses a thread-safe event callback to deliver AccessKit
    /// events to the main event loop. The primary disadvantage of this approach
    /// is that it's not possible to synchronously return an initial tree
    /// in response to the [`WindowEvent::InitialTreeRequested`] event,
    /// so some platform adapters will have to use a temporary placeholder tree
    /// until you send the first update. For an optimal implementation,
    /// consider using [`Adapter::with_direct_handlers`] or
    /// [`Adapter::with_mixed_handlers`] instead.
    ///
    /// # Panics
    ///
    /// Panics if the window is already visible.
    pub fn with_event_handler<F: Fn(Event) + Send + Sync + 'static>(
        event_loop: &dyn ActiveEventLoop,
        window: &dyn Window,
        callback: F,
    ) -> Self {
        let callback = std::sync::Arc::new(callback);
        let window_id = window.id();
        let activation_handler = WinitActivationHandler {
            window_id,
            callback: callback.clone(),
        };
        let action_handler = WinitActionHandler {
            window_id,
            callback: callback.clone(),
        };
        let deactivation_handler = WinitDeactivationHandler {
            window_id,
            callback,
        };
        Self::with_direct_handlers(
            event_loop,
            window,
            activation_handler,
            action_handler,
            deactivation_handler,
        )
    }

    /// Creates a new AccessKit adapter for a winit window. This must be done
    /// before the window is shown for the first time. This means that you must
    /// use [`winit::window::WindowAttributes::with_visible`] to make the window
    /// initially invisible, then create the adapter, then show the window.
    ///
    /// Use this if you want to provide your own AccessKit handler callbacks
    /// rather than dispatching requests through the application event loop. This is
    /// especially useful for the activation handler, because depending on
    /// your application's architecture, implementing the handler directly may
    /// allow you to return an initial tree synchronously, rather than requiring
    /// some platform adapters to use a placeholder tree until you send
    /// the first update. However, remember that each of these handlers may be
    /// called on any thread, depending on the underlying platform adapter.
    ///
    /// # Panics
    ///
    /// Panics if the window is already visible.
    pub fn with_direct_handlers(
        event_loop: &dyn ActiveEventLoop,
        window: &dyn Window,
        activation_handler: impl 'static + ActivationHandler + Send,
        action_handler: impl 'static + ActionHandler + Send,
        deactivation_handler: impl 'static + DeactivationHandler + Send,
    ) -> Self {
        if window.is_visible() == Some(true) {
            panic!(
                "The AccessKit winit adapter must be created before the window is shown (made visible) for the first time."
            );
        }

        let inner = platform_impl::Adapter::new(
            event_loop,
            window,
            activation_handler,
            action_handler,
            deactivation_handler,
        );
        Self { inner }
    }

    /// Creates a new AccessKit adapter for a winit window. This must be done
    /// before the window is shown for the first time. This means that you must
    /// use [`winit::window::WindowAttributes::with_visible`] to make the window
    /// initially invisible, then create the adapter, then show the window.
    ///
    /// This constructor provides a mix of the approaches used by
    /// [`Adapter::with_event_handler`] and [`Adapter::with_direct_handlers`].
    /// It uses the callback for the action request and deactivation
    /// events, which can be handled asynchronously with no drawback,
    /// while using a direct, caller-provided activation handler that can
    /// return the initial tree synchronously. Remember that the thread on which
    /// the activation handler is called is platform-dependent.
    ///
    /// # Panics
    ///
    /// Panics if the window is already visible.
    pub fn with_mixed_handlers<F: Fn(Event) + Send + Sync + 'static>(
        event_loop: &dyn ActiveEventLoop,
        window: &dyn Window,
        activation_handler: impl 'static + ActivationHandler + Send,
        callback: F,
    ) -> Self {
        let callback = std::sync::Arc::new(callback);
        let window_id = window.id();
        let action_handler = WinitActionHandler {
            window_id,
            callback: callback.clone(),
        };
        let deactivation_handler = WinitDeactivationHandler {
            window_id,
            callback,
        };
        Self::with_direct_handlers(
            event_loop,
            window,
            activation_handler,
            action_handler,
            deactivation_handler,
        )
    }

    /// Allows reacting to window events.
    ///
    /// This must be called whenever a new window event is received
    /// and before it is handled by the application.
    pub fn process_event(&mut self, window: &dyn Window, event: &WinitWindowEvent) {
        self.inner.process_event(window, event);
    }

    /// If and only if the tree has been initialized, call the provided function
    /// and apply the resulting update. Note: If the caller's implementation of
    /// [`ActivationHandler::request_initial_tree`] initially returned `None`,
    /// or if the caller created the adapter using an event callback, then
    /// the [`TreeUpdate`] returned by the provided function must contain
    /// a full tree.
    pub fn update_if_active(&mut self, updater: impl FnOnce() -> TreeUpdate) {
        self.inner.update_if_active(updater);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn callbacks_keep_window_identity_and_action_payload_across_threads() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let output = Arc::clone(&events);
        let callback = Arc::new(move |event| output.lock().unwrap().push(event));
        // This ID is only an inert message label; no platform API receives it.
        let window_id = WindowId::from_raw(42);
        let request = ActionRequest {
            action: accesskit::Action::Focus,
            target_tree: accesskit::TreeId::ROOT,
            target_node: accesskit::NodeId(7),
            data: None,
        };
        let expected = request.clone();
        std::thread::spawn(move || {
            let mut activation = WinitActivationHandler {
                window_id,
                callback: Arc::clone(&callback),
            };
            let mut action = WinitActionHandler {
                window_id,
                callback: Arc::clone(&callback),
            };
            let mut deactivation = WinitDeactivationHandler {
                window_id,
                callback: callback,
            };
            assert!(activation.request_initial_tree().is_none());
            action.do_action(request);
            deactivation.deactivate_accessibility();
        })
        .join()
        .unwrap();
        let events = events.lock().unwrap();
        assert!(events.iter().all(|event| event.window_id == window_id));
        assert_eq!(
            events
                .iter()
                .map(|event| event.window_event.clone())
                .collect::<Vec<_>>(),
            vec![
                WindowEvent::InitialTreeRequested,
                WindowEvent::ActionRequested(expected),
                WindowEvent::AccessibilityDeactivated,
            ]
        );
    }
}
