extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod data;

use data::{ Blockstate, Model, Element };

use std::env;
use std::ops::{ Add, Sub };
use std::fs::{ File, OpenOptions };
use std::io::{ self, BufReader, Write };

static VERSION: &'static str = "0.1.0";

struct AABox(f32, f32, f32, f32, f32, f32);

impl AABox {
    fn from(from: &Vec<f32>, to: &Vec<f32>) -> AABox {
        if from.len() != 3 { panic!("Not a 3D-vector"); }
        if to.len() != 3 { panic!("Not a 3D-vector"); }

        AABox(from[0], from[1], from[2], to[0], to[1], to[2])
    }
}

enum Axis {
    X, Y, Z
}

impl Axis {
    fn from(s: &str) -> Axis {
        match s {
            "x" | "X" => Axis::X,
            "y" | "Y" => Axis::Y,
            "z" | "Z" => Axis::Z,
            _ => panic!("Invalid axis"),
        }
    }
}

#[derive(Clone, Copy)]
struct Vec3(f32, f32, f32);

impl Vec3 {
    fn from(vec: &Vec<f32>) -> Vec3 {
        if vec.len() != 3 { panic!("Not a 3D-vector"); }

        Vec3(vec[0], vec[1], vec[2])
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }
}

struct ModelRotation {
    x: f32,
    y: f32,
    z: f32,
}

/// Approximates a non-axis-aligned box.
fn approximate(el: &Element, modrot: &ModelRotation) -> AABox {
    let verts = into_verts(AABox::from(&el.from, &el.to));
    let mut verts = match &el.rotation {
        Some(rot) => rotate(verts, Vec3::from(&rot.origin), Axis::from(&rot.axis), rot.angle),
        None => verts,
    };

    let origin = Vec3(8.0, 8.0, 8.0);

    // Apply model rotation based on blockstate.
    if modrot.x != 0.0 {
        verts = rotate(verts, origin, Axis::X, -modrot.x); // Assuming the - is needed
    }

    if modrot.y != 0.0 {
        verts = rotate(verts, origin, Axis::Y, -modrot.y);
    }

    if modrot.z != 0.0 {
        verts = rotate(verts, origin, Axis::Z, -modrot.z); // Assuming the - is needed
    }

    let min_x = verts.iter().min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).expect("0-size vector???").0;
    let max_x = verts.iter().max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).expect("0-size vector???").0;

    let min_y = verts.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).expect("0-size vector???").1;
    let max_y = verts.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).expect("0-size vector???").1;

    let min_z = verts.iter().min_by(|a, b| a.2.partial_cmp(&b.2).unwrap()).expect("0-size vector???").2;
    let max_z = verts.iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap()).expect("0-size vector???").2;

    AABox(min_x, min_y, min_z, max_x, max_y, max_z)
}

fn into_verts(el: AABox) -> Vec<Vec3> {
    vec![
        //0, 0, 0
        Vec3(el.0, el.1, el.2),
        //1, 0, 0
        Vec3(el.3, el.1, el.2),
        //1, 1, 0
        Vec3(el.3, el.4, el.2),
        //0, 1, 0
        Vec3(el.0, el.4, el.2),

        //0, 0, 1
        Vec3(el.0, el.1, el.5),
        //1, 0, 1
        Vec3(el.3, el.1, el.5),
        //1, 1, 1
        Vec3(el.3, el.4, el.5),
        //0, 1, 1
        Vec3(el.0, el.4, el.5),
    ]
}

fn rotate(mut el: Vec<Vec3>, origin: Vec3, axis: Axis, angle: f32) -> Vec<Vec3> {
    el.iter_mut()
        .for_each(|v| *v = *v - origin);

    el.iter_mut()
        .for_each(|v| {
            let (x, y) = match axis {
                Axis::X => (v.2, v.1),
                Axis::Y => (v.0, v.2),
                Axis::Z => (v.1, v.0),
            };

            let pi = std::f32::consts::PI;
            let de = (2.0 * pi) / 360.0;
            let angle = -angle * de;

            // Rotate CW?
            let (x, y) = (
                (x * angle.cos() - y * angle.sin()),
                (y * angle.cos() + x * angle.sin())
            );

            match axis {
                Axis::X => *v = Vec3(v.0, y, x),
                Axis::Y => *v = Vec3(x, v.1, y),
                Axis::Z => *v = Vec3(y, x, v.2),
            }
        });
    el.iter_mut()
        .for_each(|v| *v = *v + origin);

    el
}

#[allow(dead_code)]
struct Style {
    start_indent_level: u32,
    tab_width: u32,
    expand_tab: bool,
}

fn mkindent(level: u32, style: &Style) -> String {
    match style.expand_tab {
        true  => " ".repeat(level as usize * style.tab_width as usize),
        false => "\t".repeat(level as usize),
    }
}

fn format_cuboid_expr(aabox: &AABox, _style: &Style) -> String {
    format!(
        "Block.makeCuboidShape({}, {}, {}, {}, {}, {})",
        aabox.0, aabox.1, aabox.2, aabox.3, aabox.4, aabox.5)
}

fn usage() -> ! {
    println!("voxelshape-conv <blockstate file> <model file> [target java file]");
    println!("    VoxelShape Converter by Garophel");
    println!("    Version {}", VERSION);
    std::process::exit(1);
}

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() < 3 {
        println!("ERR: Wrong number of arguments!");
        usage();
    }

    let state_file = &args[1];
    let model_file = &args[2];
    let target_file = args.get(3);

    let blockstate: Blockstate = {
        let file = File::open(state_file).expect("File not found");
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).expect("Deser failed")
    };

    let model: Model = {
        let file = File::open(model_file).expect("File not found");
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).expect("Deser failed")
    };

    let out_file = target_file
        .map(|path| OpenOptions::new().write(true).truncate(true).create(true).open(path).unwrap());

    let mut out: Box<dyn Write> = match out_file {
        Some(file) => Box::new(file),
        None => Box::new(std::io::stdout()),
    };

    match code(&mut out, blockstate, model) {
        Err(e) => println!("IO error: {:?}", e),
        Ok(()) => (),
    }
}

fn code(out: &mut Write, blockstate: Blockstate, model: Model) -> io::Result<()> {
    let style = Style {
        start_indent_level: 2,
        tab_width: 4,
        expand_tab: true,
    };

    let nindent = mkindent(1, &style);
    let iindent = mkindent(2, &style);

    let count = model.elements.len();

    // "Header"
    writeln!(out, "package com.example.examplemod.block;")?;
    writeln!(out, "")?;
    writeln!(out, "// File generated by VoxelShape-Conv")?;
    writeln!(out, "//         Coded by Garophel")?;
    writeln!(out, "")?;
    writeln!(out, "import net.minecraft.block.Block;")?;
    writeln!(out, "import net.minecraft.util.Util;")?;
    writeln!(out, "import net.minecraft.util.math.shapes.VoxelShape;")?;
    writeln!(out, "import net.minecraft.util.math.shapes.VoxelShapes;")?;
    writeln!(out, "")?;
    writeln!(out, "public class TestyBlockBB {{")?;

    for variant in blockstate.variants.values() {
        let modrot = ModelRotation {
            x: variant.x.unwrap_or(0.0),
            y: variant.y.unwrap_or(0.0),
            z: variant.z.unwrap_or(0.0),
        };

        let variant_name = {
            let mut name = "BB".to_string();

            if let Some(x) = variant.x {
                name += &format!("_X{}", x as i32);
            }

            if let Some(y) = variant.y {
                name += &format!("_Y{}", y as i32);
            }

            if let Some(z) = variant.z {
                name += &format!("_Z{}", z as i32);
            }

            name
        };

        writeln!(out, "{}protected static final VoxelShape {} = Util.make(() -> {{", nindent, variant_name)?;

        for res in model.elements.iter()
            .nth(0).iter()
            .map(|e| approximate(e, &modrot))
            .map(|aabox| {
                writeln!(
                    out, "{}VoxelShape part = VoxelShapes.or({},",
                    iindent,
                    format_cuboid_expr(&aabox, &style))
            }) {
                res?;
            }

        for res in model.elements.iter()
            .skip(1) // Skip first
            .take(count.saturating_sub(2)) // Skip last
            .map(|e| approximate(e, &modrot))
            .map(|aabox| {
                writeln!(out, "{}VoxelShapes.or({},", iindent, format_cuboid_expr(&aabox, &style))
            }) {
                res?;
            }

        for res in model.elements.iter()
            .last().iter()
            .map(|e| approximate(e, &modrot))
            .map(|aabox| {
                let closepars = ")".repeat(1.max(count - 1));

                writeln!(out, "{}{}{};", iindent, format_cuboid_expr(&aabox, &style), closepars)
            }) {
                res?;
            }

        writeln!(out, "{}return part;", iindent)?;
        writeln!(out, "{}}});", nindent)?;

        writeln!(out, "")?;
    }

    // "Footer"
    writeln!(out, "}}")?;

    Ok(())
}

// .for_each(|aabox| {
//     println!(
//         "AABOX: {:6.3} -> {:6.3}, {:6.3} -> {:6.3}, {:6.3} -> {:6.3}",
//         aabox.0, aabox.3,
//         aabox.1, aabox.4,
//         aabox.2, aabox.5)
// })