use std::fmt;
use std::fs::{ File };
use std::io::{ self, BufReader, BufRead };
use std::path::{ Path, PathBuf };

// List of directories which should be ignored since nobody *should*
// put their source files there.
// NOTE: only at the root of the project.
fn blacklist_dirs() -> Vec<String> {
    vec![
        // Version control
        ".git",
        ".hg",

        // Build system
        "build",
        "gradle",
        ".gradle",
        "out",
        "run",
        "bin",

        // IDEs
        ".idea",
        "eclipse",

        // Other?
    ]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// Create a flat list of files within the project.
pub fn discover_files(project_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let blacklist = blacklist_dirs();

    let mut discovered_files = Vec::new();

    if !project_dir.is_dir() {
        return Err("Project root is not a directory".to_string());
    }

    let ents = match project_dir.read_dir() {
        Err(e) => return Err(format!("IO Err: {:?}", e)),
        Ok(ents) => ents,
    };

    for ent in ents {
        if let Ok(ent) = ent {
            let blacklisted = blacklist.iter()
                .any(|bl_dir| bl_dir.as_str() == ent.file_name());

            if blacklisted {
                continue;
            }

            let path = ent.path();

            if path.is_dir() {
                discover_files_rc(path, &mut discovered_files)?;
            }
        }
    }

    Ok(discovered_files)
}

// Recursively scan directories.
fn discover_files_rc(base: PathBuf, output: &mut Vec<PathBuf>) -> Result<(), String> {
    if !base.is_dir() {
        return Err("Project root is not a directory".to_string());
    }

    let ents = match base.read_dir() {
        Err(e) => return Err(format!("IO Err: {:?}", e)),
        Ok(ents) => ents,
    };

    for ent in ents {
        if let Ok(ent) = ent {
            let path = ent.path();

            if path.is_dir() {
                discover_files_rc(path, output)?;
            } else {
                output.push(path);
            }
        }
    }

    Ok(())
}

pub fn filter_blockmodels(path: &PathBuf) -> bool {
    filter_blockmodels_impl(path).unwrap_or(false)
}

fn filter_blockmodels_impl(path: &PathBuf) -> Option<bool> {
    let parent = path.parent()?;
    let parent_fn = parent.file_name()?;

    if parent_fn != "block" {
        return Some(false);
    }

    let grandparent = parent.parent()?;
    let grandparent_fn = grandparent.file_name()?;

    Some(if grandparent_fn == "models" {
        true
    } else {
        false
    })
}

pub fn filter_blockstates(path: &PathBuf) -> bool {
    filter_blockstates_impl(path).unwrap_or(false)
}

fn filter_blockstates_impl(path: &PathBuf) -> Option<bool> {
    let parent = path.parent()?;
    let parent_fn = parent.file_name()?;

    Some(if parent_fn == "blockstates" {
        true
    } else {
        false
    })
}

pub struct BlockInfo {
    pub path: PathBuf,
    pub package: String,
    pub classname: String,
    pub ids: Vec<String>,
    pub target: PathBuf,
    pub target_new: bool,
    pub target_next_to: bool,
}

impl fmt::Display for BlockInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "path: {:?}", self.path)?;
        writeln!(f, "package: {}", self.package)?;
        writeln!(f, "classname: {}", self.classname)?;

        let ids = self.ids.iter()
            .fold(String::new(), |acc, id| acc + ", " + id);

        if ids.len() >= 2 {
            writeln!(f, "ids: {}", &ids[2..])?;
        } else {
            writeln!(f, "ids:")?;
        }

        writeln!(f, "target: {:?} ({}) ({})", self.target,
                 if self.target_new {
                     "create"
                 } else {
                     "reuse"
                 },

                 if self.target_next_to {
                     "next_to"
                 } else {
                     "'blockshape' package"
                 }
        )?;

        Ok(())
    }
}

/// Split a package descriptor at periods '.' and
/// return it as a Vec so the last part is at the end.
fn package_into_vec(package: &str) -> Vec<String> {
    let mut stack = Vec::new();

    for part in package.split(".") {
        stack.push(part.to_string());
    }

    stack
}

fn find_bb_target(path: &Path, classname: &str, prefer_blockshape_package: bool) -> Option<(bool, PathBuf)> {
    let parent = path.parent()?;

    if !parent.is_dir() {
        return None;
    }

    let next_to = parent.join(format!("{}BB.java", classname));

    if !prefer_blockshape_package {
        // Next_to is preferred.
        return Some((true, next_to));
    } else if prefer_blockshape_package && next_to.is_file() {
        // If next_to already exists, use that.
        return Some((true, next_to));
    }

    let grandparent = parent.parent()?;
    let blockshape_package = grandparent.join("blockshape");

    if !blockshape_package.is_dir() {
        return Some((true, next_to));
    }

    Some((false, blockshape_package))
}

pub fn process_java_file(path: &Path) -> io::Result<BlockInfo> {
    let processors = java_line_processors();
    let mut ids = Vec::new();

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut package = None;
    let mut classname = None;

    for line in reader.lines() {
        let line = line?;

        if package.is_none() {
            java_package_find(&line, &mut package);
        }

        if classname.is_none() {
            java_classname_find(&line, &mut classname);
        }

        for proc in processors.iter() {
            if let Some(ref mut new_ids) = proc(&line) {
                ids.append(new_ids);
            }
        }
    }

    let package = package.ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "Unable to find package from java file"))?;

    let classname = classname.ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "Unable to find classname from java file"))?;

    let (next_to, target) = find_bb_target(path, &classname, false)
        .ok_or(io::Error::new(
            io::ErrorKind::Other,
            "Unable to find target file for VoxelShape bounding box"))?;

    let package = if next_to {
        package
    } else {
        let mut parts = package_into_vec(&package);

        if parts.last().map_or(false, |last| last.starts_with("block")) {
            parts.pop(); // .block[..]
        }

        parts.push("blockshape".to_string());

        parts.iter().rev().fold(String::new(), |acc, part| {
            if acc.is_empty() {
                acc + part
            } else {
                acc + "." + part
            }
        })
    };

    let target_new = !target.exists();

    let binfo = BlockInfo {
        path: path.to_path_buf(),
        package: package,
        classname: classname,
        ids: ids,
        target: target,
        target_new: target_new,
        target_next_to: next_to,
    };

    Ok(binfo)
}

fn java_package_find(line: &str, out: &mut Option<String>) -> Option<()> {
    let marker = "package";

    let start = line.find(marker)? + marker.len();
    let end = line.find(";")?;

    if end < start {
        return None;
    }

    let package = &line[start..end].trim();

    out.replace(package.to_string());
    Some(())
}

static WHITESPACE: &'static str = " \t\n\r";

fn first_non_ws(line: &str) -> Option<usize> {
    for (i, b) in line.bytes().enumerate() {
        if !WHITESPACE.as_bytes().iter().any(|w| *w == b) {
            return Some(i);
        }
    }

    None
}

fn first_ws(line: &str) -> Option<usize> {
    for (i, b) in line.bytes().enumerate() {
        if WHITESPACE.as_bytes().iter().any(|w| *w == b) {
            return Some(i);
        }
    }

    Some(line.len())
}

fn java_classname_find(line: &str, out: &mut Option<String>) -> Option<()> {
    let marker = "class";

    let start = line.find(marker)? + marker.len();
    let start = first_non_ws(&line[start..])? + start;

    let end = first_ws(&line[start..])? + start;

    if end < start {
        return None;
    }

    let classname = &line[start..end].trim();

    out.replace(classname.to_string());
    Some(())
}

fn java_line_processors() -> Vec<Box<Fn(&str) -> Option<Vec<String>>>> {
    vec![
        Box::new(java_field_ids),
        Box::new(java_field_id),
        Box::new(java_comment_ids),
    ]
}

fn java_comment_ids(line: &str) -> Option<Vec<String>> {
    let marker = "VSC! BLOCK_ID";

    if !line.contains(marker) {
        return None;
    }

    let beg = line.find(marker)? + marker.len();

    if beg >= line.len() {
        return None;
    }

    read_quoted_ids(&line[beg..])
}

fn java_field_ids(line: &str) -> Option<Vec<String>> {
    if !line.contains("VSC_BLOCK_IDS") {
        return None;
    }

    let beg = line.find("{")?;
    let end = line.find("}")?;

    if end < beg || end - beg <= 1 {
        return None;
    }

    read_quoted_ids(&line[beg..end])
}

fn java_field_id(line: &str) -> Option<Vec<String>> {
    if !line.contains("VSC_BLOCK_ID") || line.contains("VSC_BLOCK_IDS") {
        return None;
    }

    let beg = line.find("=")?;

    read_quoted_ids(&line[beg..])
}

fn find_dquots(s: &str) -> Option<(usize, usize)> {
    let start = s.find("\"")?;

    if start >= s.len() - 1 {
        return None;
    }

    let end = s[(start+1)..].find("\"")? + start + 1;

    Some((start, end))
}

fn read_quoted_ids(line: &str) -> Option<Vec<String>> {
    let mut ids = Vec::new();
    let mut start = 0;
    loop {
        let slice = &line[start..];

        match find_dquots(slice) {
            Some((a, b)) => {
                let id = &slice[a+1..b];

                if !id.is_empty() {
                    ids.push(id.to_string());
                }

                start = start + b + 1;

                if start >= line.len() {
                    return Some(ids);
                }
            },
            None => return Some(ids),
        }
    }
}
