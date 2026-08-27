// Copyright (c) the JPEG XL Project Authors. All rights reserved.
//
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use divan::Bencher;
use jxl::api::JxlDecoderOptions;
use jxl_cli::dec::{DecodeOutput, OutputDataType, decode_frames};

fn main() {
    divan::main();
}

fn decode_f32(mut input: &[u8]) -> DecodeOutput {
    decode_frames(
        &mut input,
        JxlDecoderOptions::default(),
        None,
        Some(OutputDataType::F32),
        &[OutputDataType::F32],
        true,
        false,
        None,
        false,
    )
    .expect("benchmark fixture must decode")
    .0
}

fn sampled_checksum(output: &DecodeOutput) -> u64 {
    output
        .frames
        .iter()
        .enumerate()
        .flat_map(|(frame_index, frame)| {
            frame
                .channels
                .iter()
                .enumerate()
                .map(move |(channel_index, channel)| {
                    let (row_bytes, height) = channel.byte_size();
                    let y = (frame_index * 17 + channel_index * 31) % height;
                    let row = channel.row(y);
                    let samples = [0, row_bytes / 2, row_bytes - 1];
                    samples.into_iter().fold(0_u64, |checksum, x| {
                        checksum
                            .wrapping_mul(0x100_0000_01b3)
                            .wrapping_add(row[x] as u64 + 1)
                    })
                })
        })
        .fold(0_u64, |checksum, value| checksum.rotate_left(9) ^ value)
}

fn bench_decode(
    bencher: Bencher,
    input: &'static [u8],
    expected_size: (usize, usize),
    expected_frames: Option<usize>,
) {
    // Keep the oracle outside the measured closure. It protects fixture identity,
    // output selection, dimensions, and animation traversal.
    let smoke = decode_f32(input);
    assert_eq!(smoke.size, expected_size);
    assert_eq!(smoke.data_type, OutputDataType::F32);
    if let Some(expected_frames) = expected_frames {
        assert_eq!(smoke.frames.len(), expected_frames);
    } else {
        assert!(smoke.frames.len() > 1);
    }
    assert_ne!(sampled_checksum(&smoke), 0);

    bencher.with_inputs(|| input).bench_local_values(decode_f32);
}

mod decode {
    use super::*;

    pub mod modular {
        use super::*;

        const GREEN_QUEEN: &[u8] =
            include_bytes!("../../jxl/resources/test/green_queen_modular_e3.jxl");

        #[divan::bench]
        fn green_queen_f32(bencher: Bencher) {
            bench_decode(bencher, GREEN_QUEEN, (438, 589), Some(1));
        }
    }

    pub mod vardct {
        use super::*;

        const GREEN_QUEEN: &[u8] =
            include_bytes!("../../jxl/resources/test/green_queen_vardct_e3.jxl");
        const BIKE: &[u8] =
            include_bytes!("../../jxl/resources/test/conformance_test_images/bike.jxl");
        const NOISE_CONFORMANCE: &[u8] =
            include_bytes!("../../jxl/resources/test/conformance_test_images/noise.jxl");
        const NOISE_MULTIPLE_LAYERS_SPLINE: &[u8] =
            include_bytes!("../../jxl/resources/test/multiple_layers_noise_spline.jxl");
        const NOISE_MINIMAL: &[u8] = include_bytes!("../../jxl/resources/test/8x8_noise.jxl");

        #[divan::bench]
        fn green_queen_f32(bencher: Bencher) {
            bench_decode(bencher, GREEN_QUEEN, (438, 589), Some(1));
        }

        #[divan::bench]
        fn bike_f32(bencher: Bencher) {
            bench_decode(bencher, BIKE, (2048, 2560), Some(1));
        }

        #[divan::bench]
        fn noise_conformance_f32(bencher: Bencher) {
            bench_decode(bencher, NOISE_CONFORMANCE, (500, 606), Some(1));
        }

        #[divan::bench]
        fn noise_multiple_layers_spline_f32(bencher: Bencher) {
            bench_decode(bencher, NOISE_MULTIPLE_LAYERS_SPLINE, (2048, 1152), Some(1));
        }

        #[divan::bench]
        fn noise_minimal_8x8_f32(bencher: Bencher) {
            bench_decode(bencher, NOISE_MINIMAL, (8, 8), Some(1));
        }
    }

    pub mod animation {
        use super::*;

        const NEWTONS_CRADLE: &[u8] = include_bytes!(
            "../../jxl/resources/test/conformance_test_images/animation_newtons_cradle.jxl"
        );

        #[divan::bench]
        fn newtons_cradle_f32(bencher: Bencher) {
            bench_decode(bencher, NEWTONS_CRADLE, (480, 360), Some(36));
        }
    }
}
