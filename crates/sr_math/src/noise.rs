//! 3D Simplex noise ported 1:1 from `Assembly-CSharp/Noise/Noise.cs` and `NoiseGen.cs`.
//!
//! Used by `EconomyDirector` to compute deterministic daily plort market price fluctuations
//! from `WorldModel.econ_seed` and day index.

const F3: f64 = 1.0 / 3.0;
const G3: f64 = 1.0 / 6.0;

#[derive(Clone, Copy)]
struct Grad {
    x: f64,
    y: f64,
    z: f64,
}

const GRAD3: [Grad; 12] = [
    Grad { x: 1.0, y: 1.0, z: 0.0 },
    Grad { x: -1.0, y: 1.0, z: 0.0 },
    Grad { x: 1.0, y: -1.0, z: 0.0 },
    Grad { x: -1.0, y: -1.0, z: 0.0 },
    Grad { x: 1.0, y: 0.0, z: 1.0 },
    Grad { x: -1.0, y: 0.0, z: 1.0 },
    Grad { x: 1.0, y: 0.0, z: -1.0 },
    Grad { x: -1.0, y: 0.0, z: -1.0 },
    Grad { x: 0.0, y: 1.0, z: 1.0 },
    Grad { x: 0.0, y: -1.0, z: 1.0 },
    Grad { x: 0.0, y: 1.0, z: -1.0 },
    Grad { x: 0.0, y: -1.0, z: -1.0 },
];

const P: [u8; 256] = [
    151, 160, 137,  91,  90,  15, 131,  13, 201,  95,  96,  53, 194, 233,   7, 225,
    140,  36, 103,  30,  69, 142,   8,  99,  37, 240,  21,  10,  23, 190,   6, 148,
    247, 120, 234,  75,   0,  26, 197,  62,  94, 252, 219, 203, 117,  35,  11,  32,
     57, 177,  33,  88, 237, 149,  56,  87, 174,  20, 125, 136, 171, 168,  68, 175,
     74, 165,  71, 134, 139,  48,  27, 166,  77, 146, 158, 231,  83, 111, 229, 122,
     60, 211, 133, 230, 220, 105,  92,  41,  55,  46, 245,  40, 244, 102, 143,  54,
     65,  25,  63, 161,   1, 216,  80,  73, 209,  76, 132, 187, 208,  89,  18, 169,
    200, 196, 135, 130, 116, 188, 159,  86, 164, 100, 109, 198, 173, 186,   3,  64,
     52, 217, 226, 250, 124, 123,   5, 202,  38, 147, 118, 126, 255,  82,  85, 212,
    207, 206,  59, 227,  47,  16,  58,  17, 182, 189,  28,  42, 223, 183, 170, 213,
    119, 248, 152,   2,  44, 154, 163,  70, 221, 153, 101, 155, 167,  43, 172,   9,
    129,  22,  39, 253,  19,  98, 108, 110,  79, 113, 224, 232, 178, 185, 112, 104,
    218, 246,  97, 228, 251,  34, 242, 193, 238, 210, 144,  12, 191, 179, 162, 241,
     81,  51, 145, 235, 249,  14, 239, 107,  49, 192, 214,  31, 181, 199, 106, 157,
    184,  84, 204, 176, 115, 121,  50,  45, 127,   4, 150, 254, 138, 236, 205,  93,
    222, 114,  67,  29,  24,  72, 243, 141, 128, 195,  78,  66, 215,  61, 156, 180,
];

const fn build_perm() -> [u8; 512] {
    let mut perm = [0u8; 512];
    let mut i = 0;
    while i < 512 {
        perm[i] = P[i & 255];
        i += 1;
    }
    perm
}

const fn build_perm_mod12() -> [u8; 512] {
    let mut mod12 = [0u8; 512];
    let mut i = 0;
    while i < 512 {
        mod12[i] = P[i & 255] % 12;
        i += 1;
    }
    mod12
}

const PERM: [u8; 512] = build_perm();
const PERM_MOD12: [u8; 512] = build_perm_mod12();

#[inline]
fn fastfloor(x: f64) -> i32 {
    let xi = x as i32;
    if x >= xi as f64 {
        xi
    } else {
        xi - 1
    }
}

#[inline]
fn dot(g: Grad, x: f64, y: f64, z: f64) -> f64 {
    g.x * x + g.y * y + g.z * z
}

/// Port of `Noise.Noise.PerlinNoise(double x, float y, float z, float scale, float height, float power)`.
pub fn perlin_noise(x: f64, y: f32, z: f32, scale: f32, height: f32, power: f32) -> f32 {
    let scale64 = scale as f64;
    let mut num = get_noise(x / scale64, (y as f64) / scale64, (z as f64) / scale64);
    num *= height;
    if power != 0.0 {
        num = num.powf(power);
    }
    num
}

/// Port of `Noise.Noise.GetNoise(double xin, double yin, double zin)`.
/// Returns values normalized into roughly `[0.0, 1.0]`.
pub fn get_noise(xin: f64, yin: f64, zin: f64) -> f32 {
    let s = (xin + yin + zin) * F3;
    let i = fastfloor(xin + s);
    let j = fastfloor(yin + s);
    let k = fastfloor(zin + s);
    let t = (i + j + k) as f64 * G3;

    let x0_origin = (i as f64) - t;
    let y0_origin = (j as f64) - t;
    let z0_origin = (k as f64) - t;
    let x0 = xin - x0_origin;
    let y0 = yin - y0_origin;
    let z0 = zin - z0_origin;

    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else if y0 < z0 {
        (0, 0, 1, 0, 1, 1)
    } else if x0 < z0 {
        (0, 1, 0, 0, 1, 1)
    } else {
        (0, 1, 0, 1, 1, 0)
    };

    let x1 = x0 - (i1 as f64) + G3;
    let y1 = y0 - (j1 as f64) + G3;
    let z1 = z0 - (k1 as f64) + G3;
    let x2 = x0 - (i2 as f64) + 2.0 * G3;
    let y2 = y0 - (j2 as f64) + 2.0 * G3;
    let z2 = z0 - (k2 as f64) + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3;
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    let ii = (i & 255) as usize;
    let jj = (j & 255) as usize;
    let kk = (k & 255) as usize;

    let gi0 = PERM_MOD12[ii + PERM[jj + PERM[kk] as usize] as usize] as usize;
    let gi1 = PERM_MOD12
        [ii + i1 as usize + PERM[jj + j1 as usize + PERM[kk + k1 as usize] as usize] as usize]
        as usize;
    let gi2 = PERM_MOD12
        [ii + i2 as usize + PERM[jj + j2 as usize + PERM[kk + k2 as usize] as usize] as usize]
        as usize;
    let gi3 = PERM_MOD12[ii + 1 + PERM[jj + 1 + PERM[kk + 1] as usize] as usize] as usize;

    let n0 = {
        let mut t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
        if t0 < 0.0 {
            0.0
        } else {
            t0 *= t0;
            t0 * t0 * dot(GRAD3[gi0], x0, y0, z0)
        }
    };
    let n1 = {
        let mut t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
        if t1 < 0.0 {
            0.0
        } else {
            t1 *= t1;
            t1 * t1 * dot(GRAD3[gi1], x1, y1, z1)
        }
    };
    let n2 = {
        let mut t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
        if t2 < 0.0 {
            0.0
        } else {
            t2 *= t2;
            t2 * t2 * dot(GRAD3[gi2], x2, y2, z2)
        }
    };
    let n3 = {
        let mut t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
        if t3 < 0.0 {
            0.0
        } else {
            t3 *= t3;
            t3 * t3 * dot(GRAD3[gi3], x3, y3, z3)
        }
    };

    (32.0 * (n0 + n1 + n2 + n3) + 1.0) as f32 * 0.5
}

/// Port of `Noise.Noise.GetOctaveNoise`.
pub fn get_octave_noise(px: f64, py: f64, pz: f64, octaves: i32) -> f32 {
    let mut sum = 0.0f32;
    let mut denom = 0.0f32;
    for i in 0..octaves {
        let weight = 0.5f64.powi(i) as f32;
        let freq = 2.0f64.powi(i) as f32 as f64;
        sum += get_noise(px * freq, py * freq, pz) * weight;
        denom += weight;
    }
    sum / denom
}

/// Port of `Noise.NoiseGen`.
#[derive(Debug, Clone, Copy)]
pub struct NoiseGen {
    pub x_scale: f64,
    pub y_scale: f64,
    pub z_scale: f64,
    pub octaves: u8,
}

impl Default for NoiseGen {
    fn default() -> Self {
        Self {
            x_scale: 0.02,
            y_scale: 0.02,
            z_scale: 1.0,
            octaves: 1,
        }
    }
}

impl NoiseGen {
    pub fn new(scale: f64, octaves: u8) -> Self {
        Self {
            x_scale: scale,
            y_scale: scale,
            z_scale: 1.0,
            octaves,
        }
    }

    pub fn with_xy_scale(x_scale: f64, y_scale: f64, octaves: u8) -> Self {
        Self {
            x_scale,
            y_scale,
            z_scale: 1.0,
            octaves,
        }
    }

    pub fn get_noise(&self, x: f64, y: f64, z: f64) -> f32 {
        if self.octaves > 1 {
            get_octave_noise(
                x * self.x_scale,
                y * self.y_scale,
                z * self.z_scale,
                self.octaves as i32,
            )
        } else {
            get_noise(x * self.x_scale, y * self.y_scale, z * self.z_scale)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_bounded() {
        let a = perlin_noise(1.0, 12345.0, -10000.0, 10.0, 0.6, 1.0);
        let b = perlin_noise(1.0, 12345.0, -10000.0, 10.0, 0.6, 1.0);
        assert_eq!(a, b);
        assert!(a >= 0.0 && a <= 0.65, "unexpected value: {a}");
    }

    #[test]
    fn perm_tables_are_valid() {
        assert_eq!(PERM[0], PERM[256]);
        assert_eq!(PERM_MOD12[42], PERM[42] % 12);
    }

    #[test]
    fn octave_noise_works() {
        let gen = NoiseGen::new(0.05, 3);
        let v = gen.get_noise(10.0, 20.0, 0.0);
        assert!(v.is_finite());
        assert!(v >= 0.0 && v <= 1.1);
    }
}
