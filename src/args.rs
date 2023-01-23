pub struct Args {
    pub file: std::path::PathBuf,
}

impl Args {
    pub fn get() -> Result<Args, String> {
        let file = std::env::args()
            .into_iter()
            .nth(1)
            .ok_or("Missing file argument.\n    Usage: ejercicio3 <PNG_PATH>".to_string())
            .map(|path| std::path::PathBuf::from(path))?;

        (file.exists() && file.is_file())
            .then(|| Args { file })
            .ok_or("Specified path is not a file or does not exist".to_string())
    }
}
