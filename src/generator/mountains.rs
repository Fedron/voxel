use noise::{
    Add, Blend, Cache, Constant, Exponent, Fbm, Max, MultiFractal, Multiply, NoiseFn, Perlin,
    RidgedMulti, ScaleBias, Seedable, Select, Turbulence,
};

use super::WorldGeneratorOptions;

#[derive(Debug, Clone, Copy)]
pub struct MountainOptions {
    pub lacunarity: f64,
    pub twist: f64,
    pub glaciation: f64,
    pub amount: f64,
}

impl MountainOptions {
    pub fn as_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let scaled_low = ScaleBias::new(self.as_low_noise(world))
            .set_scale(0.03125)
            .set_bias(-0.96875);

        let scaled_high = ScaleBias::new(self.as_high_noise(world))
            .set_scale(0.25)
            .set_bias(0.25);

        let add = Add::new(scaled_high, self.as_base_noise(world));

        let select = Select::new(scaled_low, add, self.as_base_noise(world))
            .set_bounds(-0.5, 999.5)
            .set_falloff(0.5);

        let scaled = ScaleBias::new(select).set_scale(0.8).set_bias(0.0);

        let ex = Exponent::new(scaled).set_exponent(self.glaciation);

        Cache::new(ex)
    }

    pub fn as_scaled_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let scaled = ScaleBias::new(self.as_noise(world))
            .set_scale(0.125)
            .set_bias(0.125);

        let fbm = Fbm::<Perlin>::new(world.seed + 90)
            .set_frequency(14.5)
            .set_persistence(0.5)
            .set_lacunarity(self.lacunarity)
            .set_octaves(6);

        let ex = Exponent::new(fbm).set_exponent(1.25);

        let scaled1 = ScaleBias::new(ex).set_scale(0.25).set_bias(1.0);

        let mult = Multiply::new(scaled, scaled1);

        Cache::new(mult)
    }

    pub fn as_base_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let base = RidgedMulti::<Perlin>::new(world.seed + 30)
            .set_frequency(1723.0)
            .set_lacunarity(self.lacunarity)
            .set_octaves(4);

        let base_scaled = ScaleBias::new(base).set_scale(0.5).set_bias(0.375);

        let valleys = RidgedMulti::<Perlin>::new(world.seed + 31)
            .set_frequency(367.0)
            .set_lacunarity(self.lacunarity)
            .set_octaves(1);

        let valleys_scaled = ScaleBias::new(valleys).set_scale(-2.0).set_bias(-0.5);

        let constant = Constant::new(-1.0);

        let blended = Blend::new(constant, base_scaled, valleys_scaled);

        let tu = Turbulence::<_, Perlin>::new(blended)
            .set_seed(world.seed + 32)
            .set_frequency(1337.0)
            .set_power(1.0 / 6730.0 * self.twist)
            .set_roughness(4);

        let tu = Turbulence::<_, Perlin>::new(tu)
            .set_seed(world.seed + 33)
            .set_frequency(21221.0)
            .set_power(1.0 / 120157.0 * self.twist)
            .set_roughness(6);

        Cache::new(tu)
    }

    pub fn as_high_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let base = RidgedMulti::<Perlin>::new(world.seed + 40)
            .set_frequency(2371.0)
            .set_lacunarity(self.lacunarity)
            .set_octaves(3);

        let base1 = RidgedMulti::<Perlin>::new(world.seed + 41)
            .set_frequency(2341.0)
            .set_lacunarity(self.lacunarity)
            .set_octaves(3);

        let max = Max::new(base, base1);

        let tu = Turbulence::<_, Perlin>::new(max)
            .set_seed(world.seed + 42)
            .set_frequency(31511.0)
            .set_power(1.0 / 180371.0 * self.twist)
            .set_roughness(4);

        Cache::new(tu)
    }

    pub fn as_low_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let base = RidgedMulti::<Perlin>::new(world.seed + 50)
            .set_frequency(1381.0)
            .set_lacunarity(self.lacunarity)
            .set_octaves(8);

        let base1 = RidgedMulti::<Perlin>::new(world.seed + 51)
            .set_frequency(1427.0)
            .set_lacunarity(self.lacunarity)
            .set_octaves(8);

        let mult = Multiply::new(base, base1);

        Cache::new(mult)
    }
}
