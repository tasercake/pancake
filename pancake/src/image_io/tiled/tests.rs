//! Pure cursor and process-budget contracts for TIFF decoding.

use std::cell::Cell;
use std::error::Error;
use std::io::{self, Cursor, Read, Seek, SeekFrom};

use super::{MAX_TIFF_WORKERS, PARALLEL_MIN_PIXELS, PositionalInput, lane_count};

/// Test failures include both source decoding errors and contract mismatches.
type TestResult = Result<(), Box<dyn Error>>;

/// Return a structured test failure without unwinding worker scopes.
///
/// # Errors
///
/// Reports a false condition with its contract name.
fn require(condition: bool, diagnostic: &str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(diagnostic.to_owned()).into())
    }
}

/// File positions and compressed bounds remain independent after interleaved seeks and reads.
///
/// # Errors
///
/// Reports source I/O or a cursor returning bytes from another tile.
#[test]
fn shared_cursors_keep_independent_positions_and_read_bounds() -> TestResult {
    let source = PositionalInput::with_length(Cursor::new(b"abcdEFGH1234"), 12);
    let left_span = Cell::new(Some((2, 5)));
    let right_span = Cell::new(Some((7, 11)));
    let mut left = source.cursor(&left_span);
    let mut right = source.cursor(&right_span);
    let _ = left.seek(SeekFrom::Start(2))?;
    let mut bytes = [0_u8; 2];
    left.read_exact(&mut bytes)?;
    require(&bytes == b"cd", "left cursor began at the wrong tile")?;
    let _ = right.seek(SeekFrom::End(-5))?;
    right.read_exact(&mut bytes)?;
    require(&bytes == b"H1", "right cursor began at the wrong tile")?;
    let mut left_tail = Vec::new();
    let _ = left.read_to_end(&mut left_tail)?;
    require(left_tail == b"E", "left cursor escaped its compressed span")?;
    let mut right_tail = Vec::new();
    let _ = right.read_to_end(&mut right_tail)?;
    require(
        right_tail == b"23",
        "right cursor escaped its compressed span",
    )
}

/// Host CPUs, useful tile rows and the process cap all bound the number of lanes.
///
/// # Errors
///
/// Reports unexpected parallelism or excessive lanes.
#[test]
fn lanes_respect_low_affinity_small_images_and_row_count() -> TestResult {
    for (pixels, rows, cpus, expected) in [
        (PARALLEL_MIN_PIXELS, 16, 1, 1),
        (PARALLEL_MIN_PIXELS, 16, 2, 2),
        (PARALLEL_MIN_PIXELS, 16, 128, MAX_TIFF_WORKERS),
        (PARALLEL_MIN_PIXELS, 2, 128, 2),
        (PARALLEL_MIN_PIXELS, 1, 128, 1),
        (10_000, 16, 128, 1),
    ] {
        require(
            lane_count(pixels, rows, cpus) == expected,
            "TIFF lane allocation exceeded its useful resource bounds",
        )?;
    }
    Ok(())
}
