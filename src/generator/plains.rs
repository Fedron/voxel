use noise::{Billow, Cache, MultiFractal, Multiply, NoiseFn, Perlin, ScaleBias};

use super::WorldGeneratorOptions;

#[derive(Debug, Clone, Copy)]
pub struct PlainOptions {
    pub lacunarity: f64,
}

impl PlainOptions {
    pub fn as_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
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

    pub fn as_scaled_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let scaled = ScaleBias::new(self.as_noise(world))
            .set_scale(0.00390625)
            .set_bias(0.0078125);

        Cache::new(scaled)
    }
}
