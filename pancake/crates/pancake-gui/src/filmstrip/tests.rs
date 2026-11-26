//! Bounded filmstrip request contracts independent of native event dispatch.

use std::path::PathBuf;

use super::{Mailbox, view_band};

/// Long stacks keep only an active band, and an empty strip has no request.
/// # Errors
/// Returns a violated selection bound.
#[test]
fn visible_band_is_bounded_and_contains_the_first_visible_frame() -> Result<(), &'static str> {
    let at_start = view_band(0, 4_096, 0);
    let middle = view_band(2_047, 4_096, 12);
    let wide = view_band(43, 4_096, 74);
    let capped = view_band(0, 4_096, 4_096);
    let last = view_band(4_096, 4_096, 12);
    if at_start != (0..24)
        || !middle.contains(&2_047)
        || middle.len() > 24
        || !wide.contains(&116)
        || capped.len() != 96
        || !last.contains(&4_095)
        || last.end != 4_096
        || !view_band(0, 0, 0).is_empty()
    {
        return Err("Filmstrip request escaped its visible resource bound.");
    }
    Ok(())
}

/// The decode thread consumes only the latest request after rapid navigation.
/// # Errors
/// Returns synchronization or stale-queue errors.
#[test]
fn pending_requests_coalesce_and_cancel_without_reusing_an_ordinal() -> Result<(), &'static str> {
    let mailbox = Mailbox::default();
    mailbox.submit(vec![(0, PathBuf::from("replaced.tif"))])?;
    let prior = mailbox.revision();
    mailbox.submit(vec![(0, PathBuf::from("current.tif"))])?;
    let next = mailbox.receive().ok_or("No new filmstrip request.")?;
    if next.revision == prior || next.frames != [(0, PathBuf::from("current.tif"))] {
        return Err("The stale source with a reused ordinal was retained.");
    }
    mailbox.close();
    if mailbox.receive().is_some() || mailbox.revision() == next.revision {
        return Err("Closing a filmstrip must invalidate undelivered work.");
    }
    Ok(())
}
