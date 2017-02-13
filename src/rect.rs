extern crate amethyst;

use amethyst::renderer::{VertexPosNormal};

pub fn gen_rectangle(w: f32, h: f32) -> Vec<VertexPosNormal> {
    let data: Vec<VertexPosNormal> = vec![VertexPosNormal {
                                              pos: [-w / 2., -h / 2., 0.],
                                              normal: [0., 0., 1.],
                                              tex_coord: [0., 0.],
                                          },
                                          VertexPosNormal {
                                              pos: [w / 2., -h / 2., 0.],
                                              normal: [0., 0., 1.],
                                              tex_coord: [1., 0.],
                                          },
                                          VertexPosNormal {
                                              pos: [w / 2., h / 2., 0.],
                                              normal: [0., 0., 1.],
                                              tex_coord: [1., 1.],
                                          },
                                          VertexPosNormal {
                                              pos: [w / 2., h / 2., 0.],
                                              normal: [0., 0., 1.],
                                              tex_coord: [1., 1.],
                                          },
                                          VertexPosNormal {
                                              pos: [-w / 2., h / 2., 0.],
                                              normal: [0., 0., 1.],
                                              tex_coord: [1., 1.],
                                          },
                                          VertexPosNormal {
                                              pos: [-w / 2., -h / 2., 0.],
                                              normal: [0., 0., 1.],
                                              tex_coord: [1., 1.],
                                          }];
    data
}
