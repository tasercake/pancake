//! Preserve delivered X11 button coordinates when control returns from GTK.

use std::cell::Cell;
use std::rc::Rc;

use slint::platform::{PointerEventButton, WindowEvent};
use slint::winit_030::{
    EventResult, WinitWindowAccessor as _,
    winit::event::{ElementState, MouseButton, WindowEvent as NativeEvent},
};
use slint::{ComponentHandle as _, LogicalPosition, PhysicalPosition, Window};
use slint_interpreter::ComponentInstance;
use x11rb::{protocol::xproto::ConnectionExt as _, rust_connection::RustConnection};

/// The next actual press may need a coordinate refresh after GTK.
#[derive(Debug)]
pub struct PointerHandoff {
    /// Updated only by a real native menu or dialog dismissal.
    pending: Rc<Cell<bool>>,
}

impl PointerHandoff {
    /// Retain the next pointer handoff and forward the original user input.
    #[must_use]
    pub fn install(owner: &ComponentInstance) -> Self {
        let pending = Rc::new(Cell::new(false));
        let synchronize = Rc::clone(&pending);
        let mut connection = None;
        let mut last = None;
        let mut forwarded = None;
        owner.window().on_winit_window_event(move |window, event| {
            match event {
                NativeEvent::CursorMoved { position, .. } => {
                    let logical = position.to_logical::<f32>(f64::from(window.scale_factor()));
                    last = Some(LogicalPosition::new(logical.x, logical.y));
                }
                NativeEvent::MouseInput {
                    state: ElementState::Pressed,
                    button,
                    ..
                } if synchronize.replace(false) => {
                    if let Some(position) = current_position(window, &mut connection)
                        && Some(position) != last
                        && forward_press(window, position, *button)
                    {
                        forwarded = Some((*button, position));
                        return EventResult::PreventDefault;
                    }
                }
                NativeEvent::MouseInput {
                    state: ElementState::Released,
                    button,
                    ..
                } if forwarded.as_ref().is_some_and(|(held, _)| held == button) => {
                    if let Some((held, pressed_at)) = forwarded.take() {
                        let position =
                            current_position(window, &mut connection).unwrap_or(pressed_at);
                        let _delivered =
                            window.dispatch_event_with_result(WindowEvent::PointerReleased {
                                position,
                                button: pointer_button(held),
                            });
                        return EventResult::PreventDefault;
                    }
                }
                _ => {}
            }
            EventResult::Propagate
        });
        Self { pending }
    }

    /// Restore the real owner window and ask for fresh X11 hit-test coordinates.
    pub fn restore_focus(&self, owner: &ComponentInstance) {
        if let Some(display) = gtk::gdk::Display::default() {
            display.sync();
        }
        self.pending.set(true);
        let _focused = owner
            .window()
            .with_winit_window(slint::winit_030::winit::window::Window::focus_window);
    }
}

/// Forward this actual native press at its real position and same button.
///
/// Winit's Slint adapter caches motion coordinates internally, so delivering
/// a position update alone cannot repair a subsequent stale-position press.
/// Forward the one matching release in the same path to maintain ordering.
fn forward_press(window: &Window, position: LogicalPosition, button: MouseButton) -> bool {
    window
        .dispatch_event_with_result(WindowEvent::PointerMoved { position })
        .and_then(|_moved| {
            window.dispatch_event_with_result(WindowEvent::PointerPressed {
                position,
                button: pointer_button(button),
            })
        })
        .is_ok()
}

/// Use the same mouse-button conversion as the pinned Winit backend.
const fn pointer_button(button: MouseButton) -> PointerEventButton {
    match button {
        MouseButton::Left => PointerEventButton::Left,
        MouseButton::Right => PointerEventButton::Right,
        MouseButton::Middle => PointerEventButton::Middle,
        MouseButton::Back => PointerEventButton::Back,
        MouseButton::Forward => PointerEventButton::Forward,
        MouseButton::Other(_) => PointerEventButton::Other,
    }
}

/// Query the pointer relative to the real window on the same local display.
///
/// After using a GTK window, Winit's X11 backend can deliver a button event
/// without the preceding motion and leave the cached position at the dialog.
fn current_position(
    owner: &Window,
    connection: &mut Option<RustConnection>,
) -> Option<LogicalPosition> {
    let window = u32::try_from(super::x11_owner(owner)?).ok()?;
    if connection.is_none() {
        *connection = Some(x11rb::connect(None).ok()?.0);
    }
    let reply = connection
        .as_ref()?
        .query_pointer(window)
        .ok()?
        .reply()
        .ok()?;
    reply.same_screen.then(|| {
        PhysicalPosition::new(i32::from(reply.win_x), i32::from(reply.win_y))
            .to_logical(owner.scale_factor())
    })
}
