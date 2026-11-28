use std::sync::LazyLock;

const BUCKET_BITS: u32 = 12;
const BUCKET_SIZE: u32 = 1 << BUCKET_BITS;

// The first float32 bit pattern for each successive sRGB byte, evaluated with
// the inspection profile's float32 curve and ties-to-even rounding. Keep the
// native profile deterministic across math libraries and CPU architectures.
#[rustfmt::skip]
const CANONICAL_CUTOFFS: [u32; 255] = [
    0x391f22b5, 0x39eeb40e, 0x3a46eb62, 0x3a8b3e5d, 0x3ab3070c, 0x3adacfb7, 0x3b014c33, 0x3b153089,
    0x3b2914e1, 0x3b3cf936, 0x3b50f2d1, 0x3b65fb99, 0x3b7c3404, 0x3b89d060, 0x3b962333, 0x3ba314bd,
    0x3bb0a733, 0x3bbedcb6, 0x3bcdb770, 0x3bdd3966, 0x3bed64b2, 0x3bfe3b46, 0x3c07df93, 0x3c10f918,
    0x3c1a6b36, 0x3c2436c8, 0x3c2e5cca, 0x3c38de1a, 0x3c43bba7, 0x3c4ef647, 0x3c5a8ee4, 0x3c668653,
    0x3c72dd73, 0x3c7f9511, 0x3c865703, 0x3c8d1490, 0x3c940395, 0x3c9b247b, 0x3ca277a8, 0x3ca9fd79,
    0x3cb1b654, 0x3cb9a29a, 0x3cc1c2aa, 0x3cca16e3, 0x3cd29fa4, 0x3cdb5d4d, 0x3ce45033, 0x3ced78b6,
    0x3cf6d72f, 0x3d0035fc, 0x3d051bb6, 0x3d0a1cee, 0x3d0f39d1, 0x3d14728a, 0x3d19c745, 0x3d1f382b,
    0x3d24c56b, 0x3d2a6f25, 0x3d303587, 0x3d3618b9, 0x3d3c18e5, 0x3d423633, 0x3d4870cb, 0x3d4ec8d5,
    0x3d553e79, 0x3d5bd1d5, 0x3d62831d, 0x3d69526b, 0x3d703ff3, 0x3d774bcf, 0x3d7e762c, 0x3d82df93,
    0x3d869376, 0x3d8a56cb, 0x3d8e29ae, 0x3d920c27, 0x3d95fe51, 0x3d9a0036, 0x3d9e11ee, 0x3da23384,
    0x3da66512, 0x3daaa6a1, 0x3daef84a, 0x3db35a17, 0x3db7cc20, 0x3dbc4e6b, 0x3dc0e117, 0x3dc58429,
    0x3dca37bc, 0x3dcefbd6, 0x3dd3d092, 0x3dd8b5f6, 0x3dddac1c, 0x3de2b30a, 0x3de7cadc, 0x3decf396,
    0x3df22d52, 0x3df7781a, 0x3dfcd401, 0x3e012088, 0x3e03dfb0, 0x3e06a77c, 0x3e0977f8, 0x3e0c5127,
    0x3e0f3315, 0x3e121dc6, 0x3e151145, 0x3e180d95, 0x3e1b12c3, 0x3e1e20d1, 0x3e2137cc, 0x3e2457b7,
    0x3e27809c, 0x3e2ab27c, 0x3e2ded6a, 0x3e313160, 0x3e347e73, 0x3e37d49d, 0x3e3b33ee, 0x3e3e9c68,
    0x3e420e18, 0x3e4588fa, 0x3e490d25, 0x3e4c9a8f, 0x3e50314f, 0x3e53d15c, 0x3e577ace, 0x3e5b2d9a,
    0x3e5ee9d8, 0x3e62af7d, 0x3e667ea3, 0x3e6a5742, 0x3e6e3969, 0x3e722512, 0x3e761a57, 0x3e7a192c,
    0x3e7e21a9, 0x3e8119e2, 0x3e8327ca, 0x3e853a88, 0x3e875225, 0x3e896e9f, 0x3e8b9000, 0x3e8db642,
    0x3e8fe173, 0x3e92118c, 0x3e944699, 0x3e968095, 0x3e98bf8c, 0x3e9b0377, 0x3e9d4c64, 0x3e9f9a4c,
    0x3ea1ed3b, 0x3ea4452b, 0x3ea6a229, 0x3ea9042e, 0x3eab6b47, 0x3eadd76c, 0x3eb048ad, 0x3eb2bf02,
    0x3eb53a74, 0x3eb7bb00, 0x3eba40b3, 0x3ebccb85, 0x3ebf5b83, 0x3ec1f0a6, 0x3ec48afc, 0x3ec72a7c,
    0x3ec9cf35, 0x3ecc791d, 0x3ecf2845, 0x3ed1dca2, 0x3ed49643, 0x3ed75521, 0x3eda1948, 0x3edce2b1,
    0x3edfb16b, 0x3ee2856a, 0x3ee55ec0, 0x3ee83d62, 0x3eeb2160, 0x3eee0ab0, 0x3ef0f961, 0x3ef3ed6a,
    0x3ef6e6da, 0x3ef9e5a5, 0x3efce9df, 0x3efff37e, 0x3f018146, 0x3f030b82, 0x3f049879, 0x3f062827,
    0x3f07ba93, 0x3f094fba, 0x3f0ae7a1, 0x3f0c8245, 0x3f0e1fac, 0x3f0fbfd2, 0x3f1162c0, 0x3f13086e,
    0x3f14b0e6, 0x3f165c22, 0x3f180a2b, 0x3f19baf9, 0x3f1b6e97, 0x3f1d24fe, 0x3f1ede37, 0x3f209a3c,
    0x3f225914, 0x3f241abb, 0x3f25df39, 0x3f27a688, 0x3f2970b0, 0x3f2b3dab, 0x3f2d0d83, 0x3f2ee033,
    0x3f30b5bf, 0x3f328e24, 0x3f34696a, 0x3f36478c, 0x3f382891, 0x3f3a0c73, 0x3f3bf33c, 0x3f3ddce5,
    0x3f3fc977, 0x3f41b8ea, 0x3f43ab4a, 0x3f45a08e, 0x3f4798c1, 0x3f4993da, 0x3f4b91e4, 0x3f4d92d8,
    0x3f4f96bf, 0x3f519d91, 0x3f53a759, 0x3f55b410, 0x3f57c3bf, 0x3f59d65e, 0x3f5bebf8, 0x3f5e0485,
    0x3f60200f, 0x3f623e90, 0x3f64600f, 0x3f668486, 0x3f68abff, 0x3f6ad671, 0x3f6d03e8, 0x3f6f345b,
    0x3f7167d5, 0x3f739e4d, 0x3f75d7cf, 0x3f781452, 0x3f7a53e0, 0x3f7c9671, 0x3f7edc11,
];

pub(crate) static CANONICAL_SRGB: LazyLock<SrgbEncoder> =
    LazyLock::new(|| SrgbEncoder::new(&CANONICAL_CUTOFFS).expect("valid sRGB cutoffs"));

pub(crate) struct SrgbEncoder {
    first: u32,
    last: u32,
    buckets: Vec<u32>,
}

impl SrgbEncoder {
    pub(crate) fn new(cutoffs: &[u32]) -> Option<Self> {
        if cutoffs.len() != 255
            || cutoffs[0] == 0
            || cutoffs[254] > 1_f32.to_bits()
            || cutoffs
                .windows(2)
                .any(|pair| pair[0] >> BUCKET_BITS >= pair[1] >> BUCKET_BITS)
        {
            return None;
        }
        let first = (cutoffs[0] - 1) & !(BUCKET_SIZE - 1);
        let last = cutoffs[254] | (BUCKET_SIZE - 1);
        let buckets = (first..=last)
            .step_by(BUCKET_SIZE as usize)
            .map(|start| {
                let code = cutoffs.partition_point(|&cutoff| cutoff <= start) as u32;
                let bias = cutoffs
                    .get(code as usize)
                    .map_or(0, |&cutoff| (start + BUCKET_SIZE).saturating_sub(cutoff));
                // A bucket spans at most one cutoff. Adding the low bits
                // crosses into the next byte value exactly at that cutoff.
                (code << BUCKET_BITS) + bias
            })
            .collect();
        Some(Self {
            first,
            last,
            buckets,
        })
    }

    #[inline]
    pub(crate) fn encode(&self, sample: f32) -> u8 {
        // Positive finite f32 values sort by their bit patterns. A signed
        // comparison puts all negative values below black.
        let bits = (sample.to_bits() as i32).clamp(self.first as i32, self.last as i32) as u32;
        let bucket = self.buckets[((bits - self.first) >> BUCKET_BITS) as usize];
        ((bucket + (bits & (BUCKET_SIZE - 1))) >> BUCKET_BITS) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoffs_cover_each_byte_and_clip_headroom() {
        for (byte, &cutoff) in CANONICAL_CUTOFFS.iter().enumerate() {
            assert_eq!(
                CANONICAL_SRGB.encode(f32::from_bits(cutoff - 1)),
                byte as u8
            );
            assert_eq!(
                CANONICAL_SRGB.encode(f32::from_bits(cutoff)),
                byte as u8 + 1
            );
        }
        assert_eq!(CANONICAL_SRGB.encode(-f32::MAX), 0);
        assert_eq!(CANONICAL_SRGB.encode(f32::MAX), 255);
    }
}
