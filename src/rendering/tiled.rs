extern crate gfx;
extern crate amethyst;
extern crate tiled;

use amethyst::renderer::pass::{DrawFlat, Pass};
use amethyst::renderer::{Pipeline, Scene};
use amethyst::renderer::target::GeometryBuffer;

use rendering::{ColorFormat, DepthFormat};

use gfx::traits::FactoryExt;

// Actual tilemap data that makes up the elements of the UBO.
// NOTE: It may be a bug, but it appears that
// [f32;2] won't work as UBO data. Possibly an issue with
// binding generation
gfx_defines!{
    constant TileMapData {
        data: [f32; 4] = "data",
    }

    constant ProjectionStuff {
        model: [[f32; 4]; 4] = "u_Model",
        view: [[f32; 4]; 4] = "u_View",
        proj: [[f32; 4]; 4] = "u_Proj",
    }

    constant TilemapStuff {
        world_size: [f32; 4] = "u_WorldSize",
        tilesheet_size: [f32; 4] = "u_TilesheetSize",
        offsets: [f32; 2] = "u_TileOffsets",
    }

    vertex VertexData {
        pos: [f32; 3] = "a_Pos",
        buf_pos: [f32; 2] = "a_BufPos",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<VertexData> = (),
        projection_cb: gfx::ConstantBuffer<ProjectionStuff> = "b_VsLocals",
        // tilemap stuff
        tilemap: gfx::ConstantBuffer<TileMapData> = "b_TileMap",
        tilemap_cb: gfx::ConstantBuffer<TilemapStuff> = "b_PsLocals",
        tilesheet: gfx::TextureSampler<[f32; 4]> = "t_TileSheet",
        // output
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
        out_depth: gfx::DepthTarget<DepthFormat> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl TileMapData {
    pub fn new_empty() -> TileMapData {
        TileMapData { data: [0.0, 0.0, 0.0, 0.0] }
    }
    pub fn new(data: [f32; 4]) -> TileMapData {
        TileMapData { data: data }
    }
}

pub struct TileMapPlane<R> where R: gfx::Resources {
    pub params: pipe::Data<R>,
    pub slice: gfx::Slice<R>,
    proj_stuff: ProjectionStuff,
    proj_dirty: bool,
    tm_stuff: TilemapStuff,
    tm_dirty: bool,
    pub data: Vec<TileMapData>,
}

impl<R> TileMapPlane<R> where R: gfx::Resources {
    pub fn new<F>(factory: &mut F, tilemap: &tiled::Map) -> TileMapPlane<R> where F: gfx::Factory<R> {
        let half_width = (tilemap.width * tilemap.tile_width) / 2;
        let half_height = (tilemap.height * tilemap.tile_height) / 2;

        let total_size = tilemap.width * tilemap.height;

        TileMapPlane{

        }
    }
}

pub struct DrawPass<R: gfx::Resources> {
    projection: gfx::handle::Buffer<R, ProjectionStuff>,
    tilemap_stuff: gfx::handle::Buffer<R, TilemapStuff>,
    tilemap_data: gfx::handle::Buffer<R, TileMapData>,
    tilesheet_sampler: gfx::handle::Sampler<R>,
    pso: gfx::PipelineState<R, pipe::Meta>,
}

impl<R: gfx::Resources> DrawPass<R> {
    pub fn new<F>(factory: &mut F) -> DrawPass<R>
        where F: gfx::Factory<R>
    {
        let sampler = factory.create_sampler(
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Scale,
                gfx::texture::WrapMode::Clamp
            )
        );

        let vert_src = include_bytes!("shader/tilemap_150.glslv");
        let frag_src = include_bytes!("shader/tilemap_150.glslf");

        DrawPass {
            projection: factory.create_constant_buffer(1),
            tilemap_stuff: factory.create_constant_buffer(1),
            tilemap_data: factory.create_constant_buffer(1),
            tilesheet_sampler: sampler,
            pso: factory.create_pipeline_simple(vert_src, frag_src, pipe::new()).unwrap(),
        }
    }
}

impl<R> Pass<R> for DrawPass<R>
    where R: gfx::Resources
{
    type Arg = DrawFlat;
    type Target = GeometryBuffer<R>;

    fn apply<C>(&self,
        _: &DrawFlat,
        _: &GeometryBuffer<R>,
        _: &Pipeline,
        scene: &Scene<R>,
        encoder: &mut gfx::Encoder<R, C>)
    where C: gfx::CommandBuffer<R>
    {
        encoder.update_constant_buffer(&self.projection, &ProjectionStuff {
            // use identity matrix until i figure out how i want to do map transforms
            model: [[1f32, 0f32, 0f32, 0f32], [0f32, 1f32, 0f32, 0f32], [0f32, 0f32, 1f32, 0f32], [0f32, 0f32, 0f32, 1f32]],
            proj: scene.camera.proj,
            view: scene.camera.view,
        });

        encoder.draw()
    }
}