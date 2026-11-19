//! Byte-level fixtures for the application `NumPy` input contract.

use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use pancake::image_io::{SampleEncoding, decode};

/// A test result retaining filesystem and decoder diagnostics.
type TestResult = Result<(), Box<dyn Error>>;

/// Write an independently formed `NumPy` header and payload.
/// # Errors
/// Returns a file-write or header-size conversion error.
fn write_array(path: &Path, version: u8, header: &[u8], payload: &[u8]) -> TestResult {
    let mut file = File::create(path)?;
    file.write_all(b"\x93NUMPY")?;
    file.write_all(&[version, 0])?;
    if version == 1 {
        file.write_all(&u16::try_from(header.len())?.to_le_bytes())?;
    } else {
        file.write_all(&u32::try_from(header.len())?.to_le_bytes())?;
    }
    file.write_all(header)?;
    file.write_all(payload)?;
    Ok(())
}

/// Pad the header of a version 2 or 3 array to the standard 64-byte boundary.
fn pad_extended_header(dictionary: &str) -> String {
    let padding = 63_usize.saturating_sub(dictionary.len().saturating_add(12) % 64);
    format!("{dictionary}{}\n", " ".repeat(padding))
}

/// Report a direct, recoverable failure from a test oracle.
/// # Errors
/// Returns the supplied diagnostic if the condition is false.
fn require(condition: bool, diagnostic: impl Into<String>) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(diagnostic.into()).into())
    }
}

/// A valid dictionary may reorder keys and omit the final comma.
/// # Errors
/// Returns an unexpected rejection, changed sample or fixture error.
#[test]
fn extended_versions_decode_reordered_metadata_without_trailing_commas() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("samples.npy");
    let big_endian_float = [-0.25_f32, f32::from_bits(0x3f00_0001), 512.0];
    let little_endian_codes = [0.0_f32, 49.0, 255.0];
    for (version, dictionary, payload, encoding, expected) in [
        (
            2,
            r#"{'fortran_order': False, 'shape': (1, 2, 1), "descr": "<u2"}"#,
            vec![1, 0, 2, 1],
            SampleEncoding::U16,
            [1.0_f32 / 65_535.0; 3]
                .into_iter()
                .chain([258.0_f32 / 65_535.0; 3])
                .collect::<Vec<_>>(),
        ),
        (
            3,
            "{'descr': '>f4', 'shape': (1, 1, 3), 'fortran_order': False}",
            big_endian_float
                .into_iter()
                .flat_map(f32::to_be_bytes)
                .collect(),
            SampleEncoding::Float,
            big_endian_float.to_vec(),
        ),
        (
            3,
            "{'shape': (1, 1, 3), 'fortran_order': False, 'descr': '<f4'}",
            little_endian_codes
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect(),
            SampleEncoding::Float,
            vec![0.0, 49.0 / 255.0, 1.0],
        ),
    ] {
        let header = pad_extended_header(dictionary);
        write_array(&path, version, header.as_bytes(), &payload)?;
        let frame = decode(&path)?;
        require(
            frame.sample_encoding() == encoding
                && frame.height() == 1
                && frame.width().saturating_mul(3) == expected.len(),
            format!("v{version} lost dimensions or source precision for {dictionary}"),
        )?;
        require(
            frame
                .samples()
                .iter()
                .copied()
                .map(f32::to_bits)
                .eq(expected.into_iter().map(f32::to_bits)),
            format!("v{version} changed decoded channel values for {dictionary}"),
        )?;
    }
    Ok(())
}

/// Require the decoder to name the source and actual failing format boundary.
/// # Errors
/// Returns unexpected success or a diagnostic mismatch.
fn require_rejection(path: &Path, needle: &str) -> TestResult {
    let failure = decode(path)
        .err()
        .ok_or_else(|| io::Error::other(format!("decoded malformed input {path:?}")))?;
    require(
        failure.contains(needle) && failure.contains(&*path.to_string_lossy()),
        format!("expected {needle:?} in source diagnostic: {failure}"),
    )
}

/// Malformed headers are rejected before discovering or reading pixel values.
/// # Errors
/// Returns unexpected acceptance, wrong error class, or fixture setup errors.
#[test]
fn invalid_metadata_reports_the_failing_field_or_shape() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("broken-header.npy");
    for (header, reason) in [
        ("{'descr': '|u1', 'shape': (1, 2)}\n", "fortran_order"),
        ("{'descr': '|u1', 'fortran_order': False}\n", "shape"),
        ("{'shape': (1, 2), 'fortran_order': False}\n", "descr"),
        (
            "{'descr' '|u1', 'shape': (1, 2), 'fortran_order': False}\n",
            "descr",
        ),
        (
            "{'descr': '|u1', 'fortran_order': True, 'shape': (1, 2)}\n",
            "row-major order",
        ),
        (
            "{'descr': '|u1', 'fortran_order': False, 'shape': [1, 2]}\n",
            "shape tuple",
        ),
        (
            "{'descr': '|u1', 'fortran_order': False, 'shape': (1, -2)}\n",
            "invalid digit",
        ),
        (
            "{'descr': '|u1', 'fortran_order': False, 'shape': (1, 2, 4)}\n",
            "NumPy image must be",
        ),
        (
            "{'descr': '<i4', 'fortran_order': False, 'shape': (1, 2)}\n",
            "unsupported NumPy dtype",
        ),
        (
            "{'descr': '|u1', 'fortran_order': False, 'shape': (1, 2)}",
            "header dictionary",
        ),
        (
            "'descr': '|u1', 'fortran_order': False, 'shape': (1, 2)}\n",
            "header dictionary",
        ),
        (
            "{'descr': '|u1', 'fortran_order': False, 'shape': (1, 2)\n",
            "header dictionary",
        ),
        (
            "{'descr': '|u1', 'fortran_order': False, 'shape': (0, 2)}\n",
            "nonzero",
        ),
        (
            "{'descr': '|u1', 'fortran_order': False, 'shape': (32769, 2)}\n",
            "limit",
        ),
    ] {
        write_array(&path, 1, header.as_bytes(), &[])?;
        require_rejection(&path, reason)?;
    }
    Ok(())
}

/// Read complete version 2 payloads through the inclusive metadata limit.
/// # Errors
/// Returns a fixture, decoded sample, or metadata boundary error.
#[test]
fn version_two_header_limit_is_inclusive() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("boundary.npy");
    let dictionary = b"{'descr': '<u2', 'fortran_order': False, 'shape': (1, 1, 3)}";
    let payload = [1, 0, 2, 1, 255, 255];
    let expected = [1.0_f32 / 65_535.0, 258.0_f32 / 65_535.0, 1.0_f32];
    // The aligned control has a twelve-byte prefix. At the exact metadata
    // limit, readers use the declared offset regardless of writer alignment.
    for (length, accepted) in [(65_524_usize, true), (65_536, true), (65_537, false)] {
        let mut header = dictionary.to_vec();
        header.resize(length.saturating_sub(1), b' ');
        header.push(b'\n');
        write_array(&path, 2, &header, &payload)?;
        if !accepted {
            require_rejection(&path, "metadata limit")?;
            continue;
        }
        let frame =
            decode(&path).map_err(|error| format!("v2 payload at {length} bytes: {error}"))?;
        require(
            [frame.width(), frame.height()] == [1, 1]
                && frame.sample_encoding() == SampleEncoding::U16
                && frame
                    .samples()
                    .iter()
                    .copied()
                    .map(f32::to_bits)
                    .eq(expected.into_iter().map(f32::to_bits)),
            format!("v2 header at {length} bytes changed the decoded RGB16 payload"),
        )?;
    }
    Ok(())
}

/// Binary headers and channel payloads must be complete and bounded.
/// # Errors
/// Returns unexpected acceptance, wrong error class, or fixture setup errors.
#[test]
fn truncated_header_and_channel_reads_are_recoverable() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("truncated.npy");
    for signature_and_length in [
        b"\x93NUMPY".as_slice(),
        b"\x93NUMPY\x01\x00\x08",
        b"\x93NUMPY\x02\x00\x08\x00",
        b"\x93NUMPY\x03\x00\x08\x00\x00\x00{}\n",
    ] {
        fs::write(&path, signature_and_length)?;
        require_rejection(&path, "fill whole buffer")?;
    }
    fs::write(&path, b"\x93NUMPY\x02\x00\x01\x00\x01\x00")?;
    require_rejection(&path, "metadata limit")?;
    write_array(&path, 3, b"{\xff}\n", &[])?;
    require_rejection(&path, "utf-8")?;

    let header =
        pad_extended_header("{'descr': '<u2', 'fortran_order': False, 'shape': (1, 1, 3)}");
    for bytes in [&[1_u8][..], &[1, 0, 2], &[1, 0, 2, 0, 3]] {
        write_array(&path, 2, header.as_bytes(), bytes)?;
        require_rejection(&path, "fill whole buffer")?;
    }

    let header = pad_extended_header("{'descr': '<f4', 'fortran_order': False, 'shape': (1, 1)}");
    write_array(&path, 3, header.as_bytes(), &f32::NAN.to_le_bytes())?;
    require_rejection(&path, "nonfinite")
}
