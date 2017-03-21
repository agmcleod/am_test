extern crate gfx;

mod tiled;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub use self::tiled::*;

pub struct Target<R: gfx::Resources> {
    pub color: gfx::handle::RenderTargetView<R, ColorFormat>,
    /// Primary depth-stencil render target.
    pub depth: gfx::handle::DepthStencilView<R, DepthFormat>,
}