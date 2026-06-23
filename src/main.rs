use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process;

use nialang::driver::pipeline;

const LANGUAGE_VERSION: &str = "0.1.0";
const MANIFEST_FILE: &str = "Niani.toml";
const MAIN_FILE: &str = "main.nia";

fn main() {
    match run_cli(env::args().skip(1)) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }
}

fn run_cli<I, S>(args: I) -> Result<i32, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.first().is_some_and(|arg| arg == "niani") {
        args.remove(0);
    }

    let mut args = args.into_iter();
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
            new_project(Path::new(&path))?;
            Ok(0)
        }
        "init" => {
            if let Some(extra) = args.next() {
                return Err(format!("unexpected argument `{extra}`\n{}", usage()));
            }
            init_project(&env::current_dir().map_err(|e| e.to_string())?)?;
            Ok(0)
        }
        "run" => {
            if let Some(extra) = args.next() {
                return Err(format!("unexpected argument `{extra}`\n{}", usage()));
            }
            run_project(&env::current_dir().map_err(|e| e.to_string())?)
        }
        "-h" | "--help" | "help" => {
            println!("{}", usage());
            Ok(0)
        }
        other => Err(format!("unknown command `{other}`\n{}", usage())),
    }
}

fn usage() -> String {
    "usage:\n  niani new <path>\n  niani init\n  niani run\n  niani help".to_string()
}

fn new_project(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("project path must not be empty".into());
    }
    if path.exists() {
        return Err(format!("destination `{}` already exists", path.display()));
    }

    let name = project_name(path)?;
    write_project_files(path, &name)?;

    println!("created niani project `{name}` at {}", path.display());
    Ok(())
}

fn init_project(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!(
            "project directory `{}` does not exist",
            path.display()
        ));
    }

    let name = project_name(path)?;
    write_project_files(path, &name)?;

    println!("initialized niani project `{name}` at {}", path.display());
    Ok(())
}

fn write_project_files(path: &Path, name: &str) -> Result<(), String> {
    let manifest = path.join(MANIFEST_FILE);
    if manifest.exists() {
        return Err(format!(
            "niani project already exists at {}",
            path.display()
        ));
    }

    let entry = path.join("src").join(MAIN_FILE);
    if entry.exists() {
        return Err(format!("entry point `{}` already exists", entry.display()));
    }

    fs::create_dir_all(path.join("src")).map_err(|e| format!("{}: {e}", path.display()))?;
    fs::write(&manifest, manifest_text(name))
        .map_err(|e| format!("{}: {e}", manifest.display()))?;
    fs::write(&entry, main_text()).map_err(|e| format!("{}: {e}", entry.display()))?;
    Ok(())
}

fn run_project(project_dir: &Path) -> Result<i32, String> {
    let manifest = project_dir.join(MANIFEST_FILE);
    if !manifest.is_file() {
        return Err(format!(
            "`{}` not found in {}; run `niani run` from a niani project root",
            MANIFEST_FILE,
            project_dir.display()
        ));
    }

    let entry = project_dir.join("src").join(MAIN_FILE);
    if !entry.is_file() {
        return Err(format!(
            "entry point `{}` not found",
            project_dir.join("src").join(MAIN_FILE).display()
        ));
    }

    let entry = fs::canonicalize(&entry).map_err(|e| {
        format!(
            "entry point `{}`: {e}",
            project_dir.join("src").join(MAIN_FILE).display()
        )
    })?;

    pipeline::run_file(&entry)
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
    "fn main() i32 {\n    println(\"Hello World!\");\n    0\n}\n"
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
    fn init_project_creates_manifest_and_main_in_current_dir() {
        let path = temp_project_path("hello_init");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("project dir");

        init_project(&path).expect("init project");

        let manifest = fs::read_to_string(path.join(MANIFEST_FILE)).expect("manifest");
        let expected_name = path.file_name().and_then(OsStr::to_str).expect("name");
        assert!(
            manifest.contains(&format!("name = \"{expected_name}\"")),
            "{manifest}"
        );

        let main = fs::read_to_string(path.join("src").join(MAIN_FILE)).expect("main");
        assert_eq!(main, main_text());

        fs::remove_dir_all(path).expect("cleanup");
    }

    #[test]
    fn init_project_rejects_existing_manifest() {
        let path = temp_project_path("existing_init");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("project dir");
        fs::write(path.join(MANIFEST_FILE), manifest_text("existing_init")).expect("manifest");

        let err = init_project(&path).expect_err("existing project");
        assert!(err.contains("already exists"), "{err}");

        fs::remove_dir_all(path).expect("cleanup");
    }

    #[test]
    fn run_cli_accepts_cargo_subcommand_prefix() {
        run_cli(["niani", "help"]).expect("cargo niani help");
    }

    #[test]
    fn run_cli_requires_new_path() {
        let err = run_cli(["new"]).expect_err("missing path");
        assert!(err.contains("niani new <path>"), "{err}");
    }

    #[test]
    fn run_project_requires_manifest() {
        let path = temp_project_path("no_manifest");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("project dir");

        let err = run_project(&path).expect_err("missing manifest");
        assert!(err.contains(MANIFEST_FILE), "{err}");

        fs::remove_dir_all(path).expect("cleanup");
    }

    #[test]
    fn run_project_requires_main_entrypoint() {
        let path = temp_project_path("no_main");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("project dir");
        fs::write(path.join(MANIFEST_FILE), manifest_text("no_main")).expect("manifest");

        let err = run_project(&path).expect_err("missing main");
        assert!(err.contains("src/main.nia"), "{err}");

        fs::remove_dir_all(path).expect("cleanup");
    }
}
