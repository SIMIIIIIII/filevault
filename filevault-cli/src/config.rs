use std::{fs, path::PathBuf};

fn path_token() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".filevault")
        .join("token")
}

pub fn save_token(token: &str) -> std::io::Result<()> {
    let path = path_token();
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, token)
}

pub fn load_token() -> Option<String> {
    fs::read_to_string(path_token()).ok()
}
