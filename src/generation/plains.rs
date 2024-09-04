use noise::{Billow, Cache, MultiFractal, Multiply, NoiseFn, Perlin, ScaleBias};

use super::WorldGenerationOptions;

/// Options for generating plains.
#[derive(Debug, Clone, Copy)]
pub struct PlainOptions {
    /// Lacunarity of the plains generation.
    pub lacunarity: f64,
}

impl PlainOptions {
    /// Creates a noise module that defines the shape of the plains.
    pub fn as_noise_module(&self, world: &WorldGenerationOptions) -> impl NoiseFn<f64, 2> {
        let scaled = ScaleBias::new(self.base_noise_module(world))
            .set_scale(0.00390625)
            .set_bias(0.0078125);

        Cache::new(scaled)
    }

    fn base_noise_module(&self, world: &WorldGenerationOptions) -> impl NoiseFn<f64, 2> {
        let base = Billow::<Perlin>::new(world.seed + 70)
            .set_frequency(1097.5)
            .set_persistence(0.5)
            .set_lacunarity(self.lacunarity)
            .set_octaves(8);

        let scaled = ScaleBias::new(base).set_scale(0.5).set_bias(0.5);

        let base = Billow::<Perlin>::new(world.seed + 71)
            .set_frequency(1097.5)
            .set_persistence(0.5)
            .set_lacunarity(self.lacunarity)
            .set_octaves(8);

        let scaled1 = ScaleBias::new(base).set_scale(0.5).set_bias(0.5);

        let mult = Multiply::new(scaled, scaled1);

        let scaled = ScaleBias::new(mult).set_scale(2.0).set_bias(-1.0);

        Cache::new(scaled)
    }
}
