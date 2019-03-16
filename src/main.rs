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
use std::io::{ self, BufWriter, Write };
use std::str::FromStr;
use std::path::{ Path, PathBuf };
use std::collections::{ HashMap, HashSet };

use serde::Deserialize;

static VERSION: &'static str = "0.1.1";

#[derive(Clone)]
/// AABox - Axis-Aligned Box, all faces face either +-X, +-Y or +-Z.
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
    for v in el.iter_mut() {
        // Adjust v so that rotation occurs as if v was rotated around
        // point 'origin'.
        *v = *v - origin;

        // Map axis.
        let (x, y) = match axis {
            Axis::X => (v.2, v.1),
            Axis::Y => (v.0, v.2),
            Axis::Z => (v.1, v.0),
        };

        let pi = std::f32::consts::PI;
        let de = (2.0 * pi) / 360.0;

        // NOTE: this line is a result of trial and error.
        // If the format of input changes, this is likely broken.
        let angle = -angle * de;

        // The actual rotation.
        let (x, y) = (
            (x * angle.cos() - y * angle.sin()),
            (y * angle.cos() + x * angle.sin())
        );

        // Un-map axis.
        match axis {
            Axis::X => *v = Vec3(v.0, y, x),
            Axis::Y => *v = Vec3(x, v.1, y),
            Axis::Z => *v = Vec3(y, x, v.2),
        }

        // Revert the adjustment after the rotation.
        *v = *v + origin
    };

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
    // Get first command-line argument or the string ".".
    let first = env::args().nth(1).unwrap_or(".".to_string());

    if first == "-h" || first == "--help" {
        // This call never returns (exit is called).
        usage();
    }

    // The directory to scan.
    let path = PathBuf::from_str(&first).unwrap();

    match automatic(path) {
        Err(e) => eprintln!("Err: {:?}", e),
        _ => {},
    }
}

fn mcid_to_stem<'a>(id: &'a str) -> &'a str {
    let file_start = id.find(":").unwrap_or(0);
    let stem_start = id[file_start..].find("/").unwrap_or(0) + file_start;

    if stem_start > 0 {
        mcid_to_stem(&id[stem_start+1..]) // This may overflow if the id ends with a '/'
    } else {
        // println!("stem: {}", &id[stem_start..]);
        &id[stem_start..]
    }
}

// Make an arbitrary string a valid Java field name.
fn fieldify(s: &str) -> String {
    let mut result = Vec::new();

    let mut first = true;
    for c in s.chars() {
        if first {
            if !c.is_ascii_alphabetic() {
                result.push('n');
            } else if c.is_ascii_alphabetic() {
                result.push(c);
            } else {
                result.push('u');
            }
        } else {
            if c.is_ascii_alphanumeric() {
                result.push(c);
            } else {
                result.push('_');
            }
        }

        first = false;
    }

    use std::iter::FromIterator;
    String::from_iter(result)
}

fn load_files<F, T>(files: &Vec<&PathBuf>, keep: F)
                    -> Result<HashMap<String, T>, String>
where F: Fn(&str) -> bool, for<'de> T: Deserialize<'de> {
    let mut map = HashMap::new();

    for path in files.iter() {
        let stem = path.file_stem()
            .ok_or(format!("No file stem in path"))?
            .to_str()
            .ok_or(format!("File stem inconversible into UTF-8"))?
            .to_owned();

        if !keep(&stem) {
            continue;
        }

        let file = File::open(path)
            .map_err(|e| format!("{:?}", e))?;

        let t = serde_json::from_reader(file)
            .map_err(|e| format!("{:?}", e))?;

        map.insert(stem, t);
    }

    Ok(map)
}

fn ends_with_variant_index(s: &str) -> bool {
    let mut numbers = false;

    let s = s.chars().collect::<Vec<char>>();

    for i in 0 .. s.len() - 1 {
        let i = s.len() - 1 - i;

        if '0' <= s[i] && s[i] <= '9' {
            numbers = true;
        } else if numbers && s[i] == 'F' {
            return i > 0 && s[i - 1] == '_';
        } else {
            return false;
        }
    }

    false
}

fn increment_variant_index(ss: &mut String) {
    let mut val = 0;
    let s = ss.as_bytes();

    let mut numstart = 0;
    for i in 0 .. s.len() - 1 {
        let j = s.len() - 1 - i;

        if '0' as u8 <= s[j] && s[j] <= '9' as u8 {
            val += (s[j] - '0' as u8) as i32 * 10_i32.pow(i as u32);
            numstart = j;
        } else {
            break;
        }
    }

    ss.replace_range(numstart.., &format!("{}", val + 1));
}

fn automatic<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let style = Style {
        start_indent_level: 1,
        tab_width: 4,
        expand_tab: true,
    };

    // ALL files discovered in the scanned directory structure.
    // (minus blacklist in scan.rs).
    let paths = scan::discover_files(&path.as_ref()).unwrap();

    let blocks = paths.iter()
        .filter(|path| path.extension().map_or(false, |ext| ext == "java"))
        .map(|path| scan::process_java_file(path))
        .flatten()
        .filter(|binfo| binfo.ids.len() > 0)
        .collect::<Vec<BlockInfo>>();

    let model_files = paths.iter()
        .filter(|path| scan::filter_blockmodels(path))
        .collect::<Vec<&PathBuf>>();

    let blockstate_files = paths.iter()
        .filter(|path| scan::filter_blockstates(path))
        .collect::<Vec<&PathBuf>>();

    let block_ids = {
        let mut ids = blocks.iter()
            .map(|block| block.ids.iter())
            .flatten()
            .cloned()
            .collect::<Vec<String>>();

        ids.sort();
        ids.dedup();

        ids
    };

    // Vec<blockstate, block id>
    let blockstates: HashMap<String, Blockstate> = load_files(
        &blockstate_files,
        |key| block_ids.iter().any(|id| id == key))?;

    let models: HashMap<String, Model> = load_files(
        &model_files,
        |key| blockstates.values()
            .any(|state| state.variants.values()
                 .any(|variant| key == mcid_to_stem(&variant.model))))?;

    for binfo in blocks.iter() {
        let mut printed_fields = HashSet::new();

        let target = &binfo.target;
        let target_package = binfo.package.clone();
        let target_classname = binfo.classname.clone() + "BB";

        // Ensure the package / directory exists.
        if !binfo.target_next_to {
            let parent = binfo.target.parent()
                .ok_or(format!("Path doesn't contant a parent"))?;

            fs::create_dir(parent.join("blockshape"))
                .map_err(|e| format!("{:?}", e))?;
        }

        let out_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&target)
            .map_err(|e| format!("{:?}", e))?;

        let mut out: Box<dyn Write> = Box::new(BufWriter::new(out_file));

        write_header(&mut out, &target_package, &target_classname)
            .map_err(|e| format!("{:?}", e))?;

        // Iterate blockstates
        for id in binfo.ids.iter() {
            let blockstate = blockstates.get(id)
                .ok_or(format!("This should be unreachable: {}", &id))?;

            for (key, variant) in blockstate.variants.iter() {
                let rotation = ModelRotation {
                    x: variant.x.unwrap_or(0.0),
                    y: variant.y.unwrap_or(0.0),
                    z: variant.z.unwrap_or(0.0),
                };

                let field_name = {
                    let mut name = format!("{}", fieldify(id));

                    if 0.0 != rotation.x {
                        name += &format!("_X{}", rotation.x as i32);
                    }

                    if 0.0 != rotation.y {
                        name += &format!("_Y{}", rotation.y as i32);
                    }

                    if 0.0 != rotation.z {
                        name += &format!("_Z{}", rotation.z as i32);
                    }

                    while printed_fields.contains(&name) {
                        if ends_with_variant_index(&name) {
                            increment_variant_index(&mut name);
                        } else {
                            name += "_F0";
                        }
                    }

                    printed_fields.insert(name.clone());

                    name
                };

                let model = models.get(mcid_to_stem(&variant.model))
                    .ok_or(format!("This should be unreachable: {}", &variant.model))?;

                let fallback = data::almost_full_cube();

                let elements = match model.elements.as_ref() {
                    Some(els) => els,
                    None => {
                        eprintln!("{}", format!("No elements in model: {}", &variant.model));

                        &fallback.elements.as_ref().unwrap()
                    },
                };

                let elements = elements.iter()
                    .map(|el| approximate(el, &rotation))
                    .collect::<Vec<AABox>>();

                let elements = merging::merge_touching(&elements);

                let visibility = "public";

                complex_write(
                    &mut out,
                    visibility,
                    &field_name,
                    Some(&key),
                    &elements,
                    &style,
                    |aabox| format_cuboid_expr(aabox, &style)
                ).map_err(|e| format!("{:?}", e))?;
            }
        }

        write_footer(&mut out)
            .map_err(|e| format!("{:?}", e))?;
    }

    Ok(())
}

fn write_header(out: &mut Write, package: &str, classname: &str)
                   -> io::Result<()>
{
    // "Header"
    writeln!(out, "package {};", package)?;
    writeln!(out, "")?;
    writeln!(out, "// File generated by VoxelShape-Conv")?;
    writeln!(out, "//         Coded by Garophel")?;
    writeln!(out, "")?;
    writeln!(out, "import net.minecraft.block.Block;")?;
    writeln!(out, "import net.minecraft.util.Util;")?;
    writeln!(out, "import net.minecraft.util.math.shapes.VoxelShape;")?;
    writeln!(out, "import net.minecraft.util.math.shapes.VoxelShapes;")?;
    writeln!(out, "")?;
    writeln!(out, "public class {} {{", classname)?;

    Ok(())
}

fn write_footer(out: &mut Write) -> io::Result<()> {
    // "Footer"
    writeln!(out, "}}")?;
    Ok(())
}

fn complex_write<F1>(
    out: &mut Write,
    visibility: &str,
    field_name: &str,
    comment: Option<&str>,
    vec: &Vec<AABox>,
    style: &Style,
    one: F1) -> io::Result<()>
where F1: Fn(&AABox) -> String
{
    // "Normal" indent level (inside public class).
    let nindent = mkindent(1, &style);

    // "Inside" indent level (inside of {}'s).
    let iindent = mkindent(2, &style);

    write!(
        out,
        "{}{} static final VoxelShape {} = Util.make(() -> {{",
        nindent,
        visibility,
        field_name)?;

    if let Some(comment) = comment {
        writeln!(out, " // {}", comment)?;
    }

    let join = "VoxelShapes.or(";

    if vec.len() == 1 {
        writeln!(out, "{}VoxelShape part = {};", iindent, one(&vec[0]))?;
    } else {
        writeln!(out, "{}VoxelShape part = {}", iindent, join)?;
        writeln!(out, "{}{},", iindent, one(&vec[0]))?;

        for i in 1 .. vec.len() - 1 {
            write!(out, "{}{}", iindent, join)?;
            writeln!(out, "{},", one(&vec[i]))?;
        }

        let closepars = ")".repeat(1.max(vec.len() - 1));
        writeln!(out, "{}{}{};", iindent, one(&vec[vec.len() - 1]), closepars)?;
    }

    writeln!(out, "{}return part;", iindent)?;
    writeln!(out, "{}}});", nindent)?;

    writeln!(out, "")?;

    Ok(())
}
