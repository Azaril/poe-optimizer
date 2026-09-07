use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2/src");
    let lua = mlua::Lua::new();
    let version: String = lua.load("return jit.version .. ' / ' .. jit.arch").eval()?;
    println!("Runtime: {version}");
    for relative in ["HeadlessWrapper.lua", "Launch.lua", "Modules/Main.lua"] {
        let source = fs::read_to_string(root.join(relative))?;
        match lua.load(&source).set_name(relative).into_function() {
            Ok(_) => println!("Parse {relative}: OK"),
            Err(error) => println!("Parse {relative}: {error}"),
        }
    }
    Ok(())
}
