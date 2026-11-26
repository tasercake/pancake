//! Offline design assets for the native workspace.

use std::error::Error;
use std::sync::Arc;

use slint::Image;
use slint::fontique_012::{fontique, shared_collection};

/// Unmodified original artwork also used by the repository's release surfaces.
#[expect(
    clippy::large_include_file,
    reason = "the original wordmark must remain available without installation files"
)]
const WORDMARK: &[u8] = include_bytes!("../../../../assets/pancake-stacker-wordmark.png");

/// The regular and semibold weights used by the two workspace type roles.
const FONTS: [(&str, &[u8]); 4] = [
    (
        "IBM Plex Mono Regular",
        include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
    ),
    (
        "IBM Plex Mono SemiBold",
        include_bytes!("../assets/fonts/IBMPlexMono-SemiBold.ttf"),
    ),
    (
        "Space Grotesk Regular",
        include_bytes!("../assets/fonts/SpaceGrotesk-Regular.ttf"),
    ),
    (
        "Space Grotesk SemiBold",
        include_bytes!("../assets/fonts/SpaceGrotesk-SemiBold.ttf"),
    ),
];

/// Add embedded faces to Slint's active collection while keeping system fallback.
/// # Errors
/// Returns an error when an embedded font cannot be registered.
pub fn register_fonts() -> Result<(), Box<dyn Error>> {
    let mut collection = shared_collection();
    for (name, bytes) in FONTS {
        let blob = fontique::Blob::new(Arc::new(bytes));
        if collection.register_fonts(blob, None).is_empty() {
            return Err(format!("Could not register {name}.").into());
        }
    }
    Ok(())
}

/// Render the original artwork on the About pane.
/// # Errors
/// Returns a decoder error for invalid embedded art.
pub fn wordmark() -> Result<Image, Box<dyn Error>> {
    Ok(Image::load_from_data(WORDMARK, Some("png"))?)
}
