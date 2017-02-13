extern crate amethyst;

use amethyst::ecs::{Component, VecStorage};
use amethyst::renderer::{VertexPosNormal};
use rect;

pub struct Player {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub size: f32,
}

impl Player {
    pub fn new() -> Player {
        Player {
            position: [0.0, 0.0],
            velocity: [0.0, 0.0],
            size: 1.0,
        }
    }

    pub fn get_renderable_verts() -> Vec<VertexPosNormal> {
        rect::gen_rectangle(10.0, 20.0)
    }
}

impl Component for Player {
    type Storage = VecStorage<Player>;
}