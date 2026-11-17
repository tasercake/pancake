//! Negative compile fixture: Slint-generated vtable glue conflicts with forbid.

use slint::ComponentHandle;

slint::slint! {
    #[edition = "1.19"]
    export component ForbiddenMacroWindow inherits Window {
        title: "Negative compile fixture";
    }
}

/// This fixture is checked, never launched on any desktop.
///
/// # Errors
///
/// Returns native window/backend errors if someone explicitly runs the fixture.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    ForbiddenMacroWindow::new()?.run()?;
    Ok(())
}
