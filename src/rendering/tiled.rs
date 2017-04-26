extern crate gfx;
extern crate amethyst;
extern crate tiled;
extern crate genmesh;
extern crate cgmath;

use std::fmt;
use std::fmt::{Debug, Formatter};

use amethyst::renderer::pass::{DrawFlat, Pass};
use amethyst::renderer::{Pipeline, Scene};
use amethyst::renderer::target::{ColorBuffer, GeometryBuffer};
use amethyst::renderer::pass::PassDescription;
use amethyst::gfx_device::gfx_types;

use rendering;

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

type CBTarget = ColorBuffer<gfx_types::Resources>;

impl TileMapData {
    pub fn new_empty() -> TileMapData {
        TileMapData { data: [0.0, 0.0, 0.0, 0.0] }
    }
    pub fn new(data: [f32; 4]) -> TileMapData {
        TileMapData { data: data }
    }
}

pub struct TileMapPlane {
    pub params: pipe::Data<gfx_types::Resources>,
    pub slice: gfx::Slice<gfx_types::Resources>,
    proj_stuff: ProjectionStuff,
    proj_dirty: bool,
    tm_stuff: TilemapStuff,
    tm_dirty: bool,
    pub data: Vec<TileMapData>,
}

impl TileMapPlane {
    pub fn new<F>(factory: &mut F, tilemap: &tiled::Map, aspect_ratio: f32 , target: &CBTarget) -> TileMapPlane
    where F: gfx::Factory<gfx_types::Resources>
    {
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
            out_depth: target.output_depth.clone(),
        };

        // TODO: change the coords here
        let view: AffineMatrix3<f32> = Transform::look_at(
            Point3::new(0.0, 0.0, 800.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_y(),
        );

        let mut map_data = Vec::with_capacity(total_size as usize);
        for _ in 0..total_size {
            map_data.push(TileMapData::new_empty());
        }

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
            data: map_data,
        }
    }

    fn prepare_buffers<C>(&self, encoder: &mut gfx::Encoder<gfx_types::Resources, C>, update_data: bool) where C: gfx::CommandBuffer<gfx_types::Resources> {
        if update_data {
            encoder.update_buffer(&self.params.tilemap, &self.data, 0).unwrap();
        }
        if self.proj_dirty {
            encoder.update_constant_buffer(&self.params.projection_cb, &self.proj_stuff);
        }
        if self.tm_dirty {
            encoder.update_constant_buffer(&self.params.tilemap_cb, &self.tm_stuff);
        }
    }

    pub fn update_x_offset(&mut self, amt: f32) {
        self.tm_stuff.offsets[0] = amt;
        self.tm_dirty = true;
    }

    pub fn update_y_offset(&mut self, amt: f32) {
        self.tm_stuff.offsets[1] = amt;
        self.tm_dirty = true;
    }
}

fn populate_tilemap(tilemap: &mut TileMap, map_data: &tiled::Map) {
    let layers = &map_data.layers;
    for layer in layers {
        for (row, cols) in layer.tiles.iter().enumerate() {
            for col in cols {
                if *col != 0 {
                    for tileset in map_data.tilesets.iter() {
                        let image = &tileset.images[0];
                        if tileset.first_gid as usize + tileset.tiles.len() - 1 <= *col as usize {
                            let x = (*col as f32 * tilemap.tile_size) % image.width as f32;
                            let y = (row as f32 * tilemap.tile_size) % image.height as f32;
                            tilemap.set_tile(*col as usize, row, [x, y, 0.0, 0.0]);
                            break
                        }
                    }
                }
            }
        }
    }
}

pub struct TileMap {
    pub tiles: Vec<TileMapData>,
    pso: gfx::PipelineState<gfx_types::Resources, pipe::Meta>,
    tilemap_plane: TileMapPlane,
    tile_size: f32,
    tilemap_size: [usize; 2],
    charmap_size: [usize; 2],
    limit_coords: [usize; 2],
    focus_coords: [usize; 2],
    focus_dirty: bool,
}

impl TileMap {
    pub fn new<F>(map: &tiled::Map, factory: &mut F, aspect_ratio: f32, target: &CBTarget) -> TileMap
        where F: gfx::Factory<gfx_types::Resources>
    {
        let mut tiles = Vec::with_capacity((map.width * map.height) as usize);
        for _ in 0..(map.width * map.height) {
            tiles.push(TileMapData::new_empty());
        }

        TileMap {
            tiles: tiles,
            pso: factory.create_pipeline_simple(
                include_bytes!("shader/tilemap_150.glslv"),
                include_bytes!("shader/tilemap_150.glslf"),
                pipe::new()
            ).unwrap(),
            tilemap_plane: TileMapPlane::new(
                factory, map, aspect_ratio, target
            ),
            tile_size: map.tile_width as f32,
            tilemap_size: [map.width as usize, map.height as usize],
            charmap_size: [map.width as usize, map.height as usize],
            limit_coords: [0, 0],
            focus_coords: [0, 0],
            focus_dirty: false,
        }
    }

    pub fn set_focus(&mut self, focus: [usize; 2]) {
        if focus[0] <= self.limit_coords[0] && focus[1] <= self.limit_coords[1] {
            self.focus_coords = focus;
            let mut charmap_ypos = 0;
            for ypos in self.focus_coords[1] .. self.focus_coords[1]+self.charmap_size[1] {
                let mut charmap_xpos = 0;
                for xpos in self.focus_coords[0] .. self.focus_coords[0]+self.charmap_size[0] {
                    let tile_idx = (ypos * self.tilemap_size[0]) + xpos;
                    let charmap_idx = (charmap_ypos * self.charmap_size[0]) + charmap_xpos;
                    self.tilemap_plane.data[charmap_idx] = self.tiles[tile_idx];
                    charmap_xpos += 1;
                }
                charmap_ypos += 1;
            }
            self.focus_dirty = true;
        } else {
            panic!("tried to set focus to {:?} with tilemap_size of {:?}", focus, self.tilemap_size);
        }
    }

    pub fn apply_x_offset(&mut self, offset_amt: f32) {
        let mut new_offset = self.tilemap_plane.tm_stuff.offsets[0] + offset_amt;
        let curr_focus = self.focus_coords;
        let new_x = if new_offset < 0.0 {
            // move down
            if self.focus_coords[0] == 0 {
                new_offset = 0.0;
                0
            } else {
                new_offset = self.tile_size + new_offset as f32;
                self.focus_coords[0] - 1
            }
        } else if self.focus_coords[0] == self.limit_coords[0] {
            // at top, no more offset
            new_offset = 0.0;
            self.focus_coords[0]
        } else if new_offset >= self.tile_size {
            new_offset = new_offset - self.tile_size as f32;
            self.focus_coords[0] + 1
        } else {
            // no move
            self.focus_coords[0]
        };
        if new_x != self.focus_coords[0] {
            self.set_focus([new_x, curr_focus[1]]);
        }
        self.tilemap_plane.update_x_offset(new_offset);
    }
    pub fn apply_y_offset(&mut self, offset_amt: f32) {
        let mut new_offset = self.tilemap_plane.tm_stuff.offsets[1] + offset_amt;
        let curr_focus = self.focus_coords;
        let new_y = if new_offset < 0.0 {
            // move down
            if self.focus_coords[1] == 0 {
                new_offset = 0.0;
                0
            } else {
                new_offset = self.tile_size + new_offset as f32;
                self.focus_coords[1] - 1
            }
        } else if self.focus_coords[1] == (self.tilemap_size[1] - self.charmap_size[1]) {
            // at top, no more offset
            new_offset = 0.0;
            self.focus_coords[1]
        } else if new_offset >= self.tile_size {
            new_offset = new_offset - self.tile_size as f32;
            self.focus_coords[1] + 1
        } else {
            // no move
            self.focus_coords[1]
        };
        if new_y != self.focus_coords[1] {
            self.set_focus([curr_focus[0], new_y]);
        }
        self.tilemap_plane.update_y_offset(new_offset);
    }

    fn calc_idx(&self, xpos: usize, ypos: usize) -> usize {
        (ypos * self.tilemap_size[0]) + xpos
    }
    pub fn set_tile(&mut self, xpos: usize, ypos: usize, data: [f32; 4]) {
        let idx = self.calc_idx(xpos, ypos);
        self.tiles[idx] = TileMapData::new(data);
    }
}

pub struct MapDrawPass {
    projection: gfx::handle::Buffer<gfx_types::Resources, ProjectionStuff>,
    tilemap_stuff: gfx::handle::Buffer<gfx_types::Resources, TilemapStuff>,
    tilemap_data: gfx::handle::Buffer<gfx_types::Resources, TileMapData>,
    tilesheet_sampler: gfx::handle::Sampler<gfx_types::Resources>,
    tilemap: &'static TileMap,
    pso: gfx::PipelineState<gfx_types::Resources, pipe::Meta>,
}

impl Debug for MapDrawPass {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "MapDrawPass")
    }
}

impl PassDescription for MapDrawPass {}

impl MapDrawPass {
    pub fn new<F>(tilemap: &'static TileMap, factory: &mut F) -> MapDrawPass
        where F: gfx::Factory<gfx_types::Resources>
    {
        let sampler = factory.create_sampler(
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Scale,
                gfx::texture::WrapMode::Clamp
            )
        );

        let vert_src = include_bytes!("shader/tilemap_150.glslv");
        let frag_src = include_bytes!("shader/tilemap_150.glslf");

        MapDrawPass {
            projection: factory.create_constant_buffer(1),
            tilemap_stuff: factory.create_constant_buffer(1),
            tilemap_data: factory.create_constant_buffer(1),
            tilesheet_sampler: sampler,
            tilemap: tilemap,
            pso: factory.create_pipeline_simple(vert_src, frag_src, pipe::new()).unwrap(),
        }
    }
}

impl Pass<gfx_types::Resources> for MapDrawPass {
    type Arg = DrawFlat;
    type Target = GeometryBuffer<gfx_types::Resources>;

    fn apply<C>(&self,
        _: &DrawFlat,
        _: &GeometryBuffer<gfx_types::Resources>,
        _: &Pipeline,
        scene: &Scene<gfx_types::Resources>,
        encoder: &mut gfx::Encoder<gfx_types::Resources, C>)
    where C: gfx::CommandBuffer<gfx_types::Resources>
    {
        encoder.update_constant_buffer(&self.projection, &ProjectionStuff {
            // use identity matrix until i figure out how i want to do map transforms
            model: [[1f32, 0f32, 0f32, 0f32], [0f32, 1f32, 0f32, 0f32], [0f32, 0f32, 1f32, 0f32], [0f32, 0f32, 0f32, 1f32]],
            proj: scene.camera.proj,
            view: scene.camera.view,
        });

        let tilemap = self.tilemap;

        tilemap.tilemap_plane.prepare_buffers(encoder, self.tilemap.focus_dirty);

        encoder.draw(&tilemap.tilemap_plane.slice, &self.pso, &tilemap.tilemap_plane.params);
    }
}