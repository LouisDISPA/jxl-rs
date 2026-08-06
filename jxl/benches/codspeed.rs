// Copyright (c) the JPEG XL Project Authors. All rights reserved.
//
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use divan::Bencher;
use jxl::benchmarks::{BlendBenchmark, Epf1Benchmark, SplineBenchmark, XybBenchmark};

fn main() {
    divan::main();
}

mod render {
    use super::*;

    pub mod dispatch {
        use super::*;

        #[divan::bench]
        fn xyb_to_linear_4096px(bencher: Bencher) {
            let benchmark = XybBenchmark::new();
            benchmark.smoke();
            bencher
                .with_inputs(|| benchmark.input())
                .bench_local_values(|mut input| benchmark.run(&mut input));
        }

        #[divan::bench]
        fn epf1_variable_active_1024px(bencher: Bencher) {
            let benchmark = Epf1Benchmark::new();
            benchmark.smoke();
            bencher
                .with_inputs(|| benchmark.output())
                .bench_local_values(|mut output| benchmark.run(&mut output));
        }
    }
}

mod spline {
    use super::*;

    pub mod draw {
        use super::*;

        #[divan::bench]
        fn default_320x320_one_spline(bencher: Bencher) {
            let benchmark = SplineBenchmark::new();
            benchmark.smoke();
            bencher
                .with_inputs(|| benchmark.input())
                .bench_local_values(|mut input| benchmark.run(&mut input));
        }
    }
}

mod blend {
    use super::*;

    pub mod non_associated_alpha {
        use super::*;

        #[divan::bench]
        fn above_4096px(bencher: Bencher) {
            let benchmark = BlendBenchmark::new();
            benchmark.smoke();
            bencher
                .with_inputs(|| benchmark.input())
                .bench_local_values(|mut input| benchmark.run(&mut input));
        }
    }
}
