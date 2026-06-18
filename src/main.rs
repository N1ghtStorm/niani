use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process;

const LANGUAGE_VERSION: &str = "0.1.0";
const MANIFEST_FILE: &str = "Nia.toml";
const MAIN_FILE: &str = "main.nia";

fn main() {
    if let Err(err) = run_cli(env::args().skip(1)) {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run_cli<I, S>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let Some(cmd) = args.next() else {
        return Err(usage());
    };

    match cmd.as_str() {
        "new" => {
            let path = args
                .next()
                .ok_or_else(|| "usage: niani new <path>".to_string())?;
            if let Some(extra) = args.next() {
                return Err(format!("unexpected argument `{extra}`\n{}", usage()));
            }
            new_project(Path::new(&path))
        }
        "-h" | "--help" | "help" => {
            println!("{}", usage());
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n{}", usage())),
    }
}

fn usage() -> String {
    "usage:\n  niani new <path>\n  niani help".to_string()
}

fn new_project(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("project path must not be empty".into());
    }
    if path.exists() {
        return Err(format!("destination `{}` already exists", path.display()));
    }

    let name = project_name(path)?;
    fs::create_dir_all(path.join("src")).map_err(|e| format!("{}: {e}", path.display()))?;
    fs::write(path.join(MANIFEST_FILE), manifest_text(&name))
        .map_err(|e| format!("{}: {e}", path.join(MANIFEST_FILE).display()))?;
    fs::write(path.join("src").join(MAIN_FILE), main_text())
        .map_err(|e| format!("{}: {e}", path.join("src").join(MAIN_FILE).display()))?;

    println!("created niani project `{name}` at {}", path.display());
    Ok(())
}

fn project_name(path: &Path) -> Result<String, String> {
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("could not infer project name from `{}`", path.display()))?;
    if name.is_empty() {
        return Err(format!(
            "could not infer project name from `{}`",
            path.display()
        ));
    }
    if !is_valid_project_name(name) {
        return Err(format!(
            "invalid project name `{name}`; use ASCII letters, digits, `_`, or `-`"
        ));
    }
    Ok(name.to_string())
}

fn is_valid_project_name(name: &str) -> bool {
    name.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn manifest_text(name: &str) -> String {
    format!(
        "[package]\nname = \"{}\"\nlanguage_version = \"{}\"\n",
        toml_escape(name),
        LANGUAGE_VERSION
    )
}

fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn main_text() -> &'static str {
    "fn main() i32 {\n    0\n}\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("niani-test-{}-{nonce}-{name}", process::id()))
    }

    #[test]
    fn new_project_creates_manifest_and_main() {
        let path = temp_project_path("hello_nia");
        let _ = fs::remove_dir_all(&path);

        new_project(&path).expect("create project");

        let manifest = fs::read_to_string(path.join(MANIFEST_FILE)).expect("manifest");
        let expected_name = path.file_name().and_then(OsStr::to_str).expect("name");
        assert!(manifest.contains("[package]"), "{manifest}");
        assert!(
            manifest.contains(&format!("name = \"{expected_name}\"")),
            "{manifest}"
        );
        assert!(
            manifest.contains("language_version = \"0.1.0\""),
            "{manifest}"
        );

        let main = fs::read_to_string(path.join("src").join(MAIN_FILE)).expect("main");
        assert_eq!(main, main_text());

        fs::remove_dir_all(path).expect("cleanup");
    }

    #[test]
    fn new_project_rejects_existing_destination() {
        let path = temp_project_path("existing");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("existing dir");

        let err = new_project(&path).expect_err("must reject existing path");
        assert!(err.contains("already exists"), "{err}");

        fs::remove_dir_all(path).expect("cleanup");
    }

    #[test]
    fn run_cli_requires_new_path() {
        let err = run_cli(["new"]).expect_err("missing path");
        assert!(err.contains("niani new <path>"), "{err}");
    }
}
