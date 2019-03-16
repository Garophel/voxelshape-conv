use std::collections::HashMap;

pub fn cube() -> Model {
    Model {
        textures: None,
        elements: Some(vec![Element {
            from: vec![ 0.0, 0.0, 0.0 ],
            to: vec![ 16.0, 16.0, 16.0 ],
            rotation: None,
            faces: None,
        }]),
        display: None,
    }
}

pub fn almost_full_cube() -> Model {
    Model {
        textures: None,
        elements: Some(vec![Element {
            from: vec![ 1.0, 0.0, 1.0 ],
            to: vec![ 15.0, 16.0, 15.0 ],
            rotation: None,
            faces: None,
        }]),
        display: None,
    }
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Blockstate {
    pub variants: HashMap<String, Variant>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Variant {
    pub model: String,
    pub uvlock: Option<bool>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Model {
    pub textures: Option<HashMap<String, String>>,
    pub elements: Option<Vec<Element>>,
    pub display: Option<Display>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Element {
    pub from: Vec<f32>,
    pub to: Vec<f32>,
    pub rotation: Option<Rotation>,
    pub faces: Option<Faces>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Faces {
    pub down: Option<Face>,
    pub up: Option<Face>,
    pub north: Option<Face>,
    pub south: Option<Face>,
    pub west: Option<Face>,
    pub east: Option<Face>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Face {
    pub uv: Vec<f32>,
    pub texture: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Rotation {
    pub origin: Vec<f32>,
    pub axis: String,
    pub angle: f32,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Display {
    pub thirdperson_righthand: Option<Transform>,
    pub thirdperson_lefthand: Option<Transform>,
    pub firstperson_righthand: Option<Transform>,
    pub firstperson_lefthand: Option<Transform>,
    pub gui: Option<Transform>,
    pub head: Option<Transform>,
    pub ground: Option<Transform>,
    pub fixed: Option<Transform>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Transform {
    pub rotation: Option<Vec<f32>>,
    pub translation: Option<Vec<f32>>,
    pub scale: Option<Vec<f32>>,
}
