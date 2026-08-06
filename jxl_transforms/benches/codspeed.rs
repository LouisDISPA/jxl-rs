// Copyright (c) the JPEG XL Project Authors. All rights reserved.
//
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use divan::Bencher;
use jxl_transforms::{
    transform::transform_to_pixels,
    transform_map::{covered_blocks_x, covered_blocks_y, HfTransformType},
};

fn main() {
    divan::main();
}

fn coefficient(index: usize) -> f32 {
    let value = ((index * 73 + 19) % 257) as f32;
    (value - 128.0) / 128.0
}

fn input_for(transform: HfTransformType) -> (Vec<f32>, Vec<f32>) {
    let blocks = covered_blocks_x(transform) as usize * covered_blocks_y(transform) as usize;
    let elements = blocks * 64;
    let lf = (0..blocks).map(coefficient).collect();
    let transform_buffer = (0..elements).map(|i| coefficient(i + blocks)).collect();
    (lf, transform_buffer)
}

fn smoke(transform: HfTransformType) {
    let (mut lf, mut transform_buffer) = input_for(transform);
    transform_to_pixels(transform, &mut lf, &mut transform_buffer);
    assert!(transform_buffer.iter().all(|value| value.is_finite()));
    assert!(transform_buffer.iter().any(|value| *value != 0.0));
}

fn bench_transform(bencher: Bencher, transform: HfTransformType) {
    smoke(transform);
    bencher
        .with_inputs(|| input_for(transform))
        .bench_local_values(|(mut lf, mut transform_buffer)| {
            transform_to_pixels(transform, &mut lf, &mut transform_buffer);
            (lf, transform_buffer)
        });
}

mod transform {
    use super::*;

    pub mod dispatch {
        use super::*;

        #[divan::bench]
        fn dct_8x8(bencher: Bencher) {
            bench_transform(bencher, HfTransformType::DCT);
        }

        #[divan::bench]
        fn dct_64x64(bencher: Bencher) {
            bench_transform(bencher, HfTransformType::DCT64X64);
        }
    }
}
