// Copyright (c) the JPEG XL Project Authors. All rights reserved.
//
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Narrow adapters for continuous benchmarks of crate-private render stages.

use std::sync::Arc;

use crate::{
    features::{
        blending::perform_blending,
        epf::SigmaSource,
        patches::{PatchBlendMode, PatchBlending},
        spline::{Point, QuantizedSpline, Splines},
    },
    frame::color_correlation_map::ColorCorrelationParams,
    headers::extra_channels::ExtraChannelInfo,
    image::Image,
    render::{
        Channels, ChannelsMut, RenderPipelineInOutStage, RenderPipelineInPlaceStage,
        stages::{Epf1Stage, OutputColorInfo, XybStage},
    },
    util::{AtomicRefCell, SmallVec, StackOnly},
};

pub const XYB_WIDTH: usize = 4096;
pub const EPF_WIDTH: usize = 1024;
pub const SPLINE_WIDTH: usize = 320;
pub const SPLINE_HEIGHT: usize = 320;
pub const BLEND_WIDTH: usize = 4096;

pub struct XybBenchmark {
    stage: XybStage,
    template: [Vec<f32>; 3],
}

pub struct XybInput([Vec<f32>; 3]);

impl Default for XybBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl XybBenchmark {
    pub fn new() -> Self {
        let template = [
            (0..XYB_WIDTH)
                .map(|x| (x as f32 / (XYB_WIDTH - 1) as f32 - 0.5) * 0.06)
                .collect(),
            (0..XYB_WIDTH)
                .map(|x| 0.2 + 0.6 * x as f32 / (XYB_WIDTH - 1) as f32)
                .collect(),
            (0..XYB_WIDTH)
                .map(|x| 0.8 - 0.6 * x as f32 / (XYB_WIDTH - 1) as f32)
                .collect(),
        ];
        Self {
            stage: XybStage::new(0, OutputColorInfo::default()),
            template,
        }
    }

    pub fn input(&self) -> XybInput {
        XybInput(self.template.clone())
    }

    pub fn run(&self, input: &mut XybInput) -> f32 {
        let [x, y, b] = &mut input.0;
        let mut rows = [x.as_mut_slice(), y.as_mut_slice(), b.as_mut_slice()];
        self.stage
            .process_row_chunk((0, 0), XYB_WIDTH, &mut rows, None);
        rows[0][0] + rows[1][XYB_WIDTH / 2] + rows[2][XYB_WIDTH - 1]
    }

    pub fn smoke(&self) {
        let mut input = self.input();
        let checksum = self.run(&mut input);
        assert!(checksum.is_finite());
        assert_ne!(checksum, 0.0);
    }
}

pub struct Epf1Benchmark {
    stage: Epf1Stage,
    input_rows: Vec<Vec<f32>>,
}

pub struct Epf1Output([Vec<f32>; 3]);

impl Default for Epf1Benchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl Epf1Benchmark {
    pub fn new() -> Self {
        let mut sigma =
            Image::<f32>::new((EPF_WIDTH.div_ceil(8) + 2, 1)).expect("valid EPF sigma image");
        for (x, value) in sigma.row_mut(0).iter_mut().enumerate() {
            *value = -0.6 - (x % 8) as f32 * 0.04;
        }

        let input_rows = (0..3)
            .flat_map(|channel| {
                (0..5).map(move |row| {
                    (0..EPF_WIDTH + 4)
                        .map(|x| {
                            let edge = if x < EPF_WIDTH / 2 { 0.15 } else { 0.85 };
                            edge + channel as f32 * 0.07
                                + row as f32 * 0.013
                                + (x % 17) as f32 * 0.001
                        })
                        .collect()
                })
            })
            .collect();

        Self {
            stage: Epf1Stage::new(
                1.0,
                2.3 / 3.0,
                [40.0, 5.0, 3.5],
                Arc::new(AtomicRefCell::new(SigmaSource::Variable(Arc::new(sigma)))),
            ),
            input_rows,
        }
    }

    pub fn output(&self) -> Epf1Output {
        Epf1Output(core::array::from_fn(|_| vec![0.0; EPF_WIDTH]))
    }

    pub fn run(&self, output: &mut Epf1Output) -> f32 {
        let input_refs: SmallVec<&[f32], 32, StackOnly> =
            self.input_rows.iter().map(Vec::as_slice).collect();
        let input = Channels::new(input_refs, 3, 5);

        {
            let output_refs: SmallVec<&mut [f32], 8, StackOnly> =
                output.0.iter_mut().map(Vec::as_mut_slice).collect();
            let mut output_channels = ChannelsMut::new(output_refs, 3, 1);
            self.stage
                .process_row_chunk((0, 1), EPF_WIDTH, &input, &mut output_channels, None);
        }

        output.0[0][0] + output.0[1][EPF_WIDTH / 2] + output.0[2][EPF_WIDTH - 1]
    }

    pub fn smoke(&self) {
        let mut output = self.output();
        let checksum = self.run(&mut output);
        assert!(checksum.is_finite());
        assert!(output.0.iter().flatten().all(|value| value.is_finite()));
        assert!(output.0.iter().enumerate().any(|(channel, row)| {
            row.iter()
                .zip(&self.input_rows[channel * 5 + 2][2..EPF_WIDTH + 2])
                .any(|(filtered, center)| (filtered - center).abs() > 1e-7)
        }));
    }
}

pub struct SplineBenchmark {
    splines: Splines,
}

pub struct SplineInput([Vec<f32>; 3]);

impl Default for SplineBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl SplineBenchmark {
    pub fn new() -> Self {
        let mut splines = Splines::default();
        splines.quantization_adjustment = 0;
        splines.splines = vec![QuantizedSpline {
            control_points: vec![
                (109, 105),
                (-130, -261),
                (-66, 193),
                (227, -52),
                (-170, 290),
            ],
            color_dct: [
                [
                    168, 119, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    9, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0,
                ],
                [
                    -10, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0,
                ],
            ],
            sigma_dct: [
                4, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ],
        }];
        splines.starting_points = vec![Point { x: 9.0, y: 54.0 }];
        splines
            .initialize_draw_cache(
                SPLINE_WIDTH as u64,
                SPLINE_HEIGHT as u64,
                &ColorCorrelationParams::default(),
                false,
            )
            .expect("valid spline benchmark input");
        Self { splines }
    }

    pub fn input(&self) -> SplineInput {
        SplineInput(core::array::from_fn(|_| {
            vec![0.0; SPLINE_WIDTH * SPLINE_HEIGHT]
        }))
    }

    pub fn run(&self, input: &mut SplineInput) -> f32 {
        let [r, g, b] = &mut input.0;
        for y in 0..SPLINE_HEIGHT {
            let range = y * SPLINE_WIDTH..(y + 1) * SPLINE_WIDTH;
            let mut rows = [&mut r[range.clone()], &mut g[range.clone()], &mut b[range]];
            self.splines.draw_segments(&mut rows, (0, y), SPLINE_WIDTH);
        }
        r[0] + g[SPLINE_WIDTH * SPLINE_HEIGHT / 2] + b[SPLINE_WIDTH * SPLINE_HEIGHT - 1]
    }

    pub fn smoke(&self) {
        let mut input = self.input();
        self.run(&mut input);
        assert!(input.0.iter().flatten().all(|value| value.is_finite()));
        assert!(input.0.iter().flatten().any(|value| *value != 0.0));
    }
}

pub struct BlendBenchmark {
    foreground: [Vec<f32>; 4],
    background: [Vec<f32>; 4],
    color_blending: PatchBlending,
    extra_channel_blending: [PatchBlending; 1],
    extra_channel_info: [ExtraChannelInfo; 1],
}

pub struct BlendInput {
    background: [Vec<f32>; 4],
    scratch: Vec<f32>,
}

impl Default for BlendBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BlendBenchmark {
    pub fn new() -> Self {
        let mut foreground = core::array::from_fn(|channel| {
            (0..BLEND_WIDTH)
                .map(|x| ((x * (channel + 3) + channel * 29) % 1024) as f32 / 1023.0)
                .collect::<Vec<_>>()
        });
        let mut background = core::array::from_fn(|channel| {
            (0..BLEND_WIDTH)
                .map(|x| ((x * (channel + 5) + channel * 47) % 1024) as f32 / 1023.0)
                .collect::<Vec<_>>()
        });
        for x in 0..BLEND_WIDTH {
            foreground[3][x] = (x % 257) as f32 / 256.0;
            background[3][x] = 1.0 - (x % 251) as f32 / 250.0;
        }

        let blending = PatchBlending {
            mode: PatchBlendMode::BlendAbove,
            alpha_channel: 0,
            clamp: true,
        };
        Self {
            foreground,
            background,
            color_blending: blending,
            extra_channel_blending: [blending],
            extra_channel_info: [ExtraChannelInfo::benchmark_non_associated_alpha()],
        }
    }

    pub fn input(&self) -> BlendInput {
        BlendInput {
            background: self.background.clone(),
            scratch: vec![0.0; 4 * BLEND_WIDTH],
        }
    }

    pub fn run(&self, input: &mut BlendInput) -> f32 {
        let [b0, b1, b2, ba] = &mut input.background;
        let mut background = [
            b0.as_mut_slice(),
            b1.as_mut_slice(),
            b2.as_mut_slice(),
            ba.as_mut_slice(),
        ];
        let foreground = self.foreground.each_ref().map(Vec::as_slice);
        perform_blending(
            &mut background,
            &foreground,
            &self.color_blending,
            &self.extra_channel_blending,
            &self.extra_channel_info,
            &mut input.scratch,
        );
        background[0][0]
            + background[1][BLEND_WIDTH / 2]
            + background[2][BLEND_WIDTH - 1]
            + background[3][BLEND_WIDTH / 3]
    }

    pub fn smoke(&self) {
        let mut input = self.input();
        let x = BLEND_WIDTH / 3;
        let foreground_alpha = self.foreground[3][x];
        let background_alpha = self.background[3][x];
        let expected_alpha = foreground_alpha + background_alpha * (1.0 - foreground_alpha);
        let checksum = self.run(&mut input);
        assert!(checksum.is_finite());
        assert!((input.background[3][x] - expected_alpha).abs() < 1e-6);
        assert_eq!(input.scratch.len(), 4 * BLEND_WIDTH);
    }
}
