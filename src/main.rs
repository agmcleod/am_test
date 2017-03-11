extern crate amethyst;
extern crate tiled;
#[macro_use]
extern crate gfx;

use amethyst::{Application, Event, State, Trans, VirtualKeyCode, WindowEvent};
use amethyst::asset_manager::{AssetManager, DirectoryStore};
use amethyst::config::Element;
use amethyst::ecs::{World, Join, RunArg, System};
use amethyst::ecs::components::{Mesh, LocalTransform, Texture, Transform};
use amethyst::gfx_device::DisplayConfig;
use amethyst::renderer::{Pipeline, VertexPosNormal};

use std::path::Path;
use std::fs::File;
use tiled::parse;

mod loader;
mod entities;
mod rect;
mod rendering;

struct Game {
    map: tiled::Map,
}

impl State for Game {
    fn on_start(&mut self, world: &mut World, assets: &mut AssetManager, pipe: &mut Pipeline) {
        use amethyst::ecs::resources::{Camera, InputHandler, Projection, ScreenDimensions};
        use amethyst::renderer::Layer;
        use amethyst::renderer::pass::{Clear, DrawFlat};
        use amethyst::gfx_device::gfx_types;

        world.add_resource::<InputHandler>(InputHandler::new());

        let layer = Layer::new("main", vec![Clear::new([0.0, 0.0, 0.0, 1.0]), DrawFlat::new("main", "main")]);

        pipe.layers.push(layer);

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

        for tileset in self.map.tilesets.iter() {
            for image in tileset.images.iter() {
                let mut factory = assets.get_loader_mut::<gfx_types::Factory>().unwrap();
                let tiles_texture = loader::gfx_load_texture(factory, &image.source);
            }
        }

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

    let mut game = Application::build(Game{ map: map }, cfg)
        .register::<entities::Player>()
        .done();

    game.run();
}
