extern crate gfx;
extern crate amethyst;
extern crate tiled;
extern crate genmesh;
extern crate cgmath;

use amethyst::renderer::pass::{DrawFlat, Pass};
use amethyst::renderer::{Pipeline, Scene};
use amethyst::renderer::target::{GeometryBuffer};
use amethyst::gfx_device;
use amethyst::gfx_device::gfx_types;

use rendering;
use rendering::{ColorFormat, DepthFormat};

use gfx::traits::FactoryExt;
use genmesh::{Vertices, Triangulate};
use genmesh::generators::{Plane, SharedVertex, IndexedPolygon};
use loader;

use cgmath::{SquareMatrix, Matrix4, AffineMatrix3};
use cgmath::{Point3, Vector3};
use cgmath::{Transform};

// this is a value based on a max buffer size (and hence tilemap size) of 64x64
// I imagine you would have a max buffer length, with multiple TileMap instances
// of varying sizes based on current screen resolution
pub const TILEMAP_BUF_LENGTH: usize = 4096;

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
        out_color: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> =
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

pub struct TileMapPlane<R: gfx::Resources> {
    pub params: pipe::Data<R>,
    pub slice: gfx::Slice<R>,
    proj_stuff: ProjectionStuff,
    proj_dirty: bool,
    tm_stuff: TilemapStuff,
    tm_dirty: bool,
    pub data: Vec<TileMapData>,
}

impl<R: gfx::Resources> TileMapPlane<R> {
    pub fn new<F>(factory: &mut F, tilemap: &tiled::Map, aspect_ratio: f32 , target: rendering::Target<R>) -> TileMapPlane<R> where F: gfx::Factory<R> {
        let half_width = (tilemap.width * tilemap.tile_width) / 2;
        let half_height = (tilemap.height * tilemap.tile_height) / 2;

        let total_size = tilemap.width * tilemap.height;

        let plane = Plane::subdivide(tilemap.width as usize, tilemap.height as usize);

        let vertex_data: Vec<VertexData> = plane.shared_vertex_iter().map(|(raw_x, raw_y)| {
            let vertex_x = half_width as f32 * raw_x;
            let vertex_y = half_height as f32 * raw_y;

            let u_pos = (1.0 + raw_x) / 2.0;
            let v_pos = (1.0 + raw_y) / 2.0;
            let tilemap_x = (u_pos * tilemap.width as f32).floor();
            let tilemap_y = (v_pos * tilemap.height as f32).floor();

            VertexData {
                pos: [vertex_x, vertex_y, 0.0],
                buf_pos: [tilemap_x as f32, tilemap_y as f32]
            }
        }).collect();

        let index_data: Vec<u32> = plane.indexed_polygon_iter()
            .triangulate()
            .vertices()
            .map(|i| i as u32)
            .collect();

        let (vbuf, slice) = factory.create_vertex_buffer_with_slice(&vertex_data, &index_data[..]);

        let tileset = tilemap.tilesets.get(0).unwrap(); // working under the assumption i will only use one tileset
        let image = tileset.images.get(0).unwrap();
        let tiles_texture = loader::gfx_load_texture(factory, &image.source);

        let params = pipe::Data {
            vbuf: vbuf,
            projection_cb: factory.create_constant_buffer(1),
            tilemap: factory.create_constant_buffer(TILEMAP_BUF_LENGTH),
            tilemap_cb: factory.create_constant_buffer(1),
            tilesheet: (tiles_texture, factory.create_sampler_linear()),
            out_color: target.color.clone(),
            out_depth: target.depth.clone(),
        };

        // TODO: change the coords here
        let view: AffineMatrix3<f32> = Transform::look_at(
            Point3::new(0.0, 0.0, 800.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_y(),
        );

        TileMapPlane{
            slice: slice,
            params: params,
            proj_stuff: ProjectionStuff {
                model: Matrix4::identity().into(),
                view: view.mat.into(),
                proj: cgmath::perspective(cgmath::deg(60.0f32), aspect_ratio, 0.1, 4000.0).into(),
            },
            proj_dirty: true,
            tm_stuff: TilemapStuff{
                world_size: [tilemap.width as f32, tilemap.height as f32, tilemap.tile_width as f32, 0.0],
                tilesheet_size: [tileset.tile_width as f32, tileset.tile_height as f32, tileset.images[0].width as f32, tileset.images[0].height as f32],
                offsets: [0.0, 0.0],
            },
            tm_dirty: true,
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