use std::{fs, io, path::Path};

use trees::Tree;

use parser::lang;


pub fn load_dir<F: FnMut(&Tree<String>)>(dir: &Path, cb: &mut F) -> io::Result<()> {
    trace!("Processing dir: {}", dir.to_string_lossy());
    if !dir.is_dir() {
        panic!(dir.to_string_lossy().to_string().push_str("  is not directory!"));
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            load_dir(&path, cb)?;
        } else if path.extension().unwrap() == "sym" || path.extension().unwrap() == "pbl" {
            load_file(&path, cb)?;
        }
    }
    Ok(())
}

fn load_file<F: FnMut(&Tree<String>)>(file: &Path, cb: &mut F) -> io::Result<()> {
    info!("Processing file: {}", file.to_string_lossy());
    let content = fs::read_to_string(file)?;
    let states = lang::StatementsParser::new().parse(&content[..]).unwrap();
    for s in states {
        cb(&s);
    }

    Ok(())
}
