use std::collections::HashMap;

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
    pub textures: HashMap<String, String>,
    pub elements: Vec<Element>,
    pub display: Option<Display>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Element {
    pub from: Vec<f32>,
    pub to: Vec<f32>,
    pub rotation: Option<Rotation>,
    pub faces: Faces,
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
    pub thirdperson_righthand: Transform,
    pub thirdperson_lefthand: Transform,
    pub firstperson_righthand: Transform,
    pub firstperson_lefthand: Transform,
    pub gui: Transform,
    pub head: Transform,
    pub ground: Transform,
    pub fixed: Transform,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Transform {
    pub rotation: Option<Vec<f32>>,
    pub translation: Option<Vec<f32>>,
    pub scale: Option<Vec<f32>>,
}
