extern crate gfx;

mod tiled;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub use self::tiled::*;