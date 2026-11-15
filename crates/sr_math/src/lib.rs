//! Core math utilities ported 1:1 from Slime Rancher's `Noise` and `Randoms` classes.

pub mod noise;
pub mod random;

pub use glam::{Quat, Vec2, Vec3, Vec4};
pub use noise::{get_noise, get_octave_noise, perlin_noise, NoiseGen};
pub use random::Randoms;
