//! Pure host contracts run under Miri and native instrumentation.

use super::{decode_pixels, finish_readback, validate_shape};

/// Shapes must be nonempty, exactly sized and addressable by the device kernel.
///
/// # Panics
///
/// Panics if a valid shape fails or an invalid shape passes validation.
#[test]
fn shape_validation_checks_all_boundaries() {
    for (width, height, count, expected) in [
        (0, 1, 0, Err("image dimensions must be nonzero".to_owned())),
        (1, 0, 0, Err("image dimensions must be nonzero".to_owned())),
        (
            usize::MAX,
            2,
            0,
            Err("image area exceeds u32 addressing".to_owned()),
        ),
        (
            usize::MAX,
            1,
            0,
            Err("image area exceeds u32 addressing".to_owned()),
        ),
        (
            2,
            2,
            3,
            Err("pixel count does not match image dimensions".to_owned()),
        ),
        (5, 3, 15, Ok(15)),
    ] {
        assert_eq!(
            validate_shape(width, height, count),
            expected,
            "shape validation contract"
        );
    }
}

/// Device readback must preserve all u16 bits and reject corrupt results.
///
/// # Panics
///
/// Panics if sample decoding loses precision or accepts malformed bytes.
#[test]
fn readback_decoding_rejects_corrupt_values() {
    let bytes: Vec<u8> = [0_u32, 32_768, 65_535]
        .into_iter()
        .flat_map(u32::to_ne_bytes)
        .collect();
    assert_eq!(
        decode_pixels(&bytes),
        Ok(vec![0, 32_768, 65_535]),
        "full u16 readback"
    );
    assert_eq!(decode_pixels(&[]), Ok(vec![]), "empty byte buffer");
    assert_eq!(
        decode_pixels(&[0, 1, 2]),
        Err("device readback length is not a multiple of four".to_owned()),
        "truncated readback"
    );
    assert_eq!(
        decode_pixels(&65_536_u32.to_ne_bytes()),
        Err("device result exceeds u16 range".to_owned()),
        "out-of-range readback"
    );
}

/// Read failures and malformed buffers must remain recoverable at the I/O boundary.
///
/// # Panics
///
/// Panics if readback errors are lost or valid buffers are decoded incorrectly.
#[test]
fn readback_failures_keep_the_runtime_error() {
    assert_eq!(
        finish_readback(Err::<Vec<cubecl::bytes::Bytes>, _>("device lost")),
        Err("device lost".to_owned()),
        "native read errors must propagate"
    );
    assert_eq!(
        finish_readback(Ok::<_, &str>(vec![])),
        Ok(vec![]),
        "empty descriptor result"
    );
    assert_eq!(
        finish_readback(Ok::<_, &str>(vec![cubecl::bytes::Bytes::from_bytes_vec(
            vec![0]
        )])),
        Err("device readback length is not a multiple of four".to_owned()),
        "malformed readback must propagate"
    );
    assert_eq!(
        finish_readback(Ok::<_, &str>(vec![cubecl::bytes::Bytes::from_bytes_vec(
            65_535_u32.to_ne_bytes().to_vec()
        )])),
        Ok(vec![65_535]),
        "typed readback must decode"
    );
}
