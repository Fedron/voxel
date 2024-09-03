use noise::{
    Billow, Blend, Cache, Constant, Exponent, Fbm, MultiFractal, Multiply, NoiseFn, Perlin,
    RidgedMulti, ScaleBias, Seedable, Turbulence,
};

use super::WorldGeneratorOptions;

#[derive(Debug, Clone, Copy)]
pub struct HillOptions {
    pub lacunarity: f64,
    pub twist: f64,
    pub amount: f64,
}

impl HillOptions {
    pub fn as_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let base = Billow::<Perlin>::new(world.seed + 60)
            .set_frequency(1663.0)
            .set_persistence(0.5)
            .set_lacunarity(self.lacunarity)
            .set_octaves(6);

        let scaled = ScaleBias::new(base).set_scale(0.5).set_bias(0.5);

        let river_valleys = RidgedMulti::<Perlin>::new(world.seed + 61)
            .set_frequency(367.5)
            .set_lacunarity(self.lacunarity)
            .set_octaves(1);

        let scaled_river_valleys = ScaleBias::new(river_valleys).set_scale(-2.0).set_bias(-1.0);

        let constant = Constant::new(-1.0);

        let blended = Blend::new(constant, scaled_river_valleys, scaled);

        let scaled = ScaleBias::new(blended).set_scale(0.75).set_bias(-0.25);

        let ex = Exponent::new(scaled).set_exponent(1.375);

        let tu = Turbulence::<_, Perlin>::new(ex)
            .set_seed(world.seed + 62)
            .set_frequency(1531.0)
            .set_power(1.0 / 16921.0 * self.twist)
            .set_roughness(4);

        let tu = Turbulence::<_, Perlin>::new(tu)
            .set_seed(world.seed + 63)
            .set_frequency(21617.0)
            .set_power(1.0 / 117529.0 * self.twist)
            .set_roughness(6);

        Cache::new(tu)
    }

    pub fn as_scaled_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let scaled = ScaleBias::new(self.as_noise(world))
            .set_scale(0.0625)
            .set_bias(0.0625);

        let fbm = Fbm::<Perlin>::new(world.seed + 100)
            .set_frequency(13.5)
            .set_persistence(0.5)
            .set_lacunarity(self.lacunarity)
            .set_octaves(6);

        let ex = Exponent::new(fbm).set_exponent(1.25);

        let scaled1 = ScaleBias::new(ex).set_scale(0.5).set_bias(1.5);

        let mult = Multiply::new(scaled, scaled1);

        Cache::new(mult)
    }
}
