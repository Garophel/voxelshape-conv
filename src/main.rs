extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod data;
mod scan;
mod merging;

use data::{ Blockstate, Model, Element };
use scan::BlockInfo;

use std::env;
use std::ops::{ Add, Sub };
use std::fs::{ self, File, OpenOptions };
use std::io::{ self, BufReader, BufWriter, Write };
use std::str::FromStr;
use std::path::PathBuf;

static VERSION: &'static str = "0.1.1";

#[derive(Clone)]
pub struct AABox(f32, f32, f32, f32, f32, f32);

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
    println!("voxelshape-conv [project directory]");
    println!("    'project directory' is optional and can be used to change");
    println!("    which directory the program will operate on (default = cwd).");
    println!("");
    println!("    VoxelShape Converter by Garophel");
    println!("    Version {}", VERSION);

    std::process::exit(0);
}

fn main() {
    let first = env::args().nth(1).unwrap_or(".".to_string());

    if first == "-h" || first == "--help" {
        usage();
    }

    let path = PathBuf::from_str(&first).unwrap();

    let paths = scan::discover_files(&path).unwrap();

    // paths.iter()
    //     .for_each(|path| println!("> {:?}", path));

    let blocks = paths.iter()
        .filter(|path| path.extension().map_or(false, |ext| ext == "java"))
        .map(|path| scan::process_java_file(path))
        .flatten()
        .filter(|binfo| binfo.ids.len() > 0)
        // .for_each(|binfo| println!(">> {}", binfo));
        .collect::<Vec<BlockInfo>>();

    let models = paths.iter()
        .filter(|path| scan::filter_blockmodels(path))
        // .for_each(|model| println!("model> {:?}", model));
        .collect::<Vec<&PathBuf>>();

    // println!("");

    let blockstates = paths.iter()
        .filter(|path| scan::filter_blockstates(path))
        // .for_each(|state| println!("state> {:?}", state));
        .collect::<Vec<&PathBuf>>();

    let style = Style {
        start_indent_level: 1,
        tab_width: 4,
        expand_tab: true,
    };

    for block in blocks.iter() {
        for block_id in block.ids.iter() {
            let blockstate: Option<Blockstate> = blockstates.iter()
                .find(|path| path.file_stem()
                      .map(|stem| stem == block_id.as_str()).unwrap_or(false))
                .and_then(|path| File::open(path).ok())
                .map(|file| BufReader::new(file))
                .and_then(|reader| serde_json::from_reader(reader).ok());

            let blockstate = match blockstate {
                Some(blockstate) => blockstate,
                None => {
                    println!("ERR: unable to load blockstate for block '{}'", block_id);
                    std::process::exit(1);
                },
            };

            let modelgens = blockstate.variants.values().map(|variant| {
                let model: Option<Model> = models.iter()
                    .find(|path| path.file_stem()
                          .map(|stem| stem == mcid_to_stem(variant.model.as_str())).unwrap_or(false))
                    .and_then(|path| File::open(path).ok())
                    .map(|file| BufReader::new(file))
                    .and_then(|reader| serde_json::from_reader(reader).ok());
                    // .map(|reader| serde_json::from_reader(reader).unwrap());

                let model = match model {
                    Some(model) => model,
                    None => {
                        println!("ERR: unable to load model for block '{}'", block_id);
                        std::process::exit(1);
                    },
                };

                let rotation = ModelRotation {
                    x: variant.x.unwrap_or(0.0),
                    y: variant.y.unwrap_or(0.0),
                    z: variant.z.unwrap_or(0.0),
                };

                ModelGen {
                    model: model,
                    rotation: rotation,
                }
            }).collect::<Vec<ModelGen>>();

            if !block.target_next_to {
                // Ensure the package / directory exists.
                let parent = match block.target.parent() {
                    Some(parent) => parent,
                    None => {
                        println!("ERR: Failed to find directory for\
                                  blockshape package.");
                        std::process::exit(2);
                    },
                };

                if let Err(e) = fs::create_dir(parent.join("blockshape")) {
                    println!("ERR: Failed to create directory for\
                              blockshape package: {:?}", e);
                    std::process::exit(2);
                }
            }

            let file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&block.target);

            let mut out: Box<dyn Write> = match file {
                Ok(out_file) => Box::new(BufWriter::new(out_file)),
                Err(e) => {
                    println!("ERR: Failed to open file due to: {:?}", e);
                    std::process::exit(2);
                },
            };

            // let mut out: Box<dyn Write> = match out_file {
            //     Some(file) => Box::new(BufWriter::new(file)),
            //     None => Box::new(std::io::stdout()),
            // };

            match generate_file(&mut out, block, modelgens, &style) {
                Err(e) => {
                    println!("ERR: Failed to generate file due to: {:?}", e);
                    std::process::exit(2);
                },
                Ok(()) => (),
            }
        }
    }
}

fn mcid_to_stem<'a>(id: &'a str) -> &'a str {
    let file_start = id.find(":").unwrap_or(0);
    let stem_start = id[file_start..].find("/").unwrap_or(0) + file_start;

    if stem_start > 0 {
        mcid_to_stem(&id[stem_start+1..]) // This may overflow if the id ends with a '/'
    } else {
        &id[stem_start..]
    }
}

struct ModelGen {
    model: Model,
    rotation: ModelRotation,
}

fn generate_file(out: &mut Write, info: &BlockInfo, models: Vec<ModelGen>, style: &Style) -> io::Result<()> {
    let nindent = mkindent(1, &style);
    let iindent = mkindent(2, &style);

    // "Header"
    writeln!(out, "package {};", info.package)?;
    writeln!(out, "")?;
    writeln!(out, "// File generated by VoxelShape-Conv")?;
    writeln!(out, "//         Coded by Garophel")?;
    writeln!(out, "")?;
    writeln!(out, "import net.minecraft.block.Block;")?;
    writeln!(out, "import net.minecraft.util.Util;")?;
    writeln!(out, "import net.minecraft.util.math.shapes.VoxelShape;")?;
    writeln!(out, "import net.minecraft.util.math.shapes.VoxelShapes;")?;
    writeln!(out, "")?;
    writeln!(out, "public class {}BB {{", info.classname)?; // TODO: actually use proper classname?

    for variant in models {
        let elements = variant.model.elements.iter()
            .map(|e| approximate(e, &variant.rotation))
            .collect::<Vec<AABox>>();

        let elements = merging::merge_touching(&elements);

        let count = elements.len();

        let variant_name = {
            let mut name = "BB".to_string();

            if variant.rotation.x != 0.0 {
                name += &format!("_X{}", variant.rotation.x as i32);
            }

            if variant.rotation.y != 0.0 {
                name += &format!("_Y{}", variant.rotation.y as i32);
            }

            if variant.rotation.z != 0.0 {
                name += &format!("_Z{}", variant.rotation.z as i32);
            }

            name
        }; // TODO: better variant name

        let visibility = if info.target_next_to {
            "protected"
        } else {
            "public"
        };

        writeln!(out, "{}{} static final VoxelShape {} = Util.make(() -> {{", nindent, visibility, variant_name)?;

        // println!("elements: {}", variant.model.elements.len());

        if count == 1 {
            for res in elements.iter()
                .nth(0).iter()
                .map(|aabox| {
                    writeln!(
                        out, "{}VoxelShape part = {};",
                        iindent,
                        format_cuboid_expr(&aabox, &style))
                }) {
                    res?;
                }
        } else {
            for res in elements.iter()
                .nth(0).iter()
                .map(|aabox| {
                    writeln!(
                        out, "{}VoxelShape part = VoxelShapes.or({},",
                        iindent,
                        format_cuboid_expr(&aabox, &style))
                }) {
                    res?;
                }

            for res in elements.iter()
                .skip(1) // Skip first
                .take(count.saturating_sub(2)) // Skip last
                .map(|aabox| {
                    writeln!(out, "{}VoxelShapes.or({},", iindent, format_cuboid_expr(&aabox, &style))
                }) {
                    res?;
                }

            for res in elements.iter()
                .last().iter()
                .map(|aabox| {
                    let closepars = ")".repeat(1.max(count - 1));

                    writeln!(out, "{}{}{};", iindent, format_cuboid_expr(&aabox, &style), closepars)
                }) {
                    res?;
                }
        }

        writeln!(out, "{}return part;", iindent)?;
        writeln!(out, "{}}});", nindent)?;

        writeln!(out, "")?;
    }

    // "Footer"
    writeln!(out, "}}")?;

    Ok(())
}

// let args = env::args().collect::<Vec<String>>();

// if args.len() < 3 {
//     println!("ERR: Wrong number of arguments!");
//     usage();
// }

// let state_file = &args[1];
// let model_file = &args[2];
// let target_file = args.get(3);

// let blockstate: Blockstate = {
//     let file = File::open(state_file).expect("File not found");
//     let reader = BufReader::new(file);
//     serde_json::from_reader(reader).expect("Deser failed")
// };

// let model: Model = {
//     let file = File::open(model_file).expect("File not found");
//     let reader = BufReader::new(file);
//     serde_json::from_reader(reader).expect("Deser failed")
// };

// let out_file = target_file
//     .map(|path| OpenOptions::new().write(true).truncate(true).create(true).open(path).unwrap());

// let mut out: Box<dyn Write> = match out_file {
//     Some(file) => Box::new(file),
//     None => Box::new(std::io::stdout()),
// };

// match code(&mut out, blockstate, model) {
//     Err(e) => println!("IO error: {:?}", e),
//     Ok(()) => (),
// }
