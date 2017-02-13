extern crate amethyst;

use amethyst::ecs::{Component, VecStorage};

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
}

impl Component for Player {
    type Storage = VecStorage<Player>;
}