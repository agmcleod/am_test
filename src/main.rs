extern crate amethyst;
extern crate tiled;
extern crate cgmath;
extern crate genmesh;
#[macro_use]
extern crate gfx;

use amethyst::{Application, Event, State, Trans, VirtualKeyCode, WindowEvent};
use amethyst::asset_manager::{AssetManager, DirectoryStore};
use amethyst::config::Element;
use amethyst::ecs::{World, Join, RunArg, System};
use amethyst::ecs::components::{Mesh, LocalTransform, Texture, Transform};
use amethyst::gfx_device::DisplayConfig;
use amethyst::renderer::{Pipeline, VertexPosNormal};
use amethyst::gfx_device::gfx_types;

use std::path::Path;
use std::fs::File;
use tiled::parse;

mod loader;
mod entities;
mod rect;
mod rendering;

use rendering::TileMap;

struct Game {
    map: tiled::Map,
    cfg: DisplayConfig,
    tilemap_drawer: Option<TileMap>,
}

impl State for Game {
    fn on_start(&mut self, world: &mut World, assets: &mut AssetManager, pipe: &mut Pipeline) {
        use amethyst::ecs::resources::{Camera, InputHandler, Projection, ScreenDimensions};
        use amethyst::renderer::Layer;
        use amethyst::renderer::pass::{Clear, DrawFlat};
        use rendering::{MapDrawPass};
        use amethyst::renderer::target::ColorBuffer;

        world.add_resource::<InputHandler>(InputHandler::new());

        {
            let factory = assets.get_loader_mut::<amethyst::gfx_device::gfx_types::Factory>()
                .expect("Couldn't retrieve factory.");

            let main_target = pipe.targets.get("main").unwrap() as &Box<amethyst::renderer::Target>;
            let main_target = main_target.downcast_ref::<ColorBuffer<gfx_types::Resources>>().unwrap();

            let dimensions = self.cfg.dimensions.unwrap();
            self.tilemap_drawer = Some(TileMap::new(&self.map, factory, (dimensions.0 / dimensions.1) as f32, &main_target));

            let tilemap_drawer = &self.tilemap_drawer.unwrap();
            let layer = Layer::new("main", vec![
                Clear::new([0.0, 0.0, 0.0, 1.0]),
                Box::new(MapDrawPass::new(tilemap_drawer, factory)),
                DrawFlat::new("main", "main"),
            ]);

            pipe.layers.push(layer);
        }

        {
            let dim = world.read_resource::<ScreenDimensions>();
            let mut camera = world.write_resource::<Camera>();
            let aspect_ratio = dim.aspect_ratio;
            let eye = [dim.w / 2.0, dim.h / 2.0, 0.1];
            let target = [dim.w / 2.0, dim.h / 2.0, 0.];
            let up = [0., 1., 0.];

            // Get an Orthographic projection
            let proj = Projection::Orthographic {
                left: -dim.w / 2.0,
                right: dim.w / 2.0,
                bottom: -dim.h / 2.0,
                top: dim.h / 2.0,
                near: 0.0,
                far: 1.0,
            };

            camera.proj = proj;
            camera.eye = eye;
            camera.target = target;
            camera.up = up;
        }

        assets.register_asset::<Mesh>();
        assets.register_asset::<Texture>();

        assets.load_asset_from_data::<Texture, [f32; 4]>("white", [1.0, 1.0, 1.0, 1.0]);
        assets.load_asset_from_data::<Mesh, Vec<VertexPosNormal>>("player", entities::Player::get_renderable_verts());

        assets.register_store(DirectoryStore::new("./resources"));

        let square = assets.create_renderable("player", "white", "white", "white", 1.0).unwrap();

        let player = entities::Player::new();

        world.create_now()
            .with(square.clone())
            .with(player)
            .with(LocalTransform::default())
            .with(Transform::default())
            .build();
    }

    fn handle_events(&mut self, events: &[WindowEvent], world: &mut World, _: &mut AssetManager, _: &mut Pipeline) -> Trans {
        use amethyst::ecs::resources::InputHandler;

        let mut input = world.write_resource::<InputHandler>();
        input.update(events);

        for e in events {
            match **e {
                Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape)) => return Trans::Quit,
                Event::Closed => return Trans::Quit,
                _ => (),
            }
        }
        Trans::None
    }
}

fn main() {
    let path = "./resources/config.yml";
    let cfg = DisplayConfig::from_file(path).unwrap();

    let map_file = File::open(&Path::new("./resources/map.tmx")).unwrap();
    let map = parse(map_file).unwrap();

    let game = Game{ map: map, cfg: cfg.clone(), tilemap_drawer: None, };
    let mut app = Application::build(game, cfg)
        .register::<entities::Player>()
        .done();

    app.run();
}
