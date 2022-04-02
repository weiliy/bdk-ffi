use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use uniffi_bindgen;

#[derive(Debug, PartialEq)]
pub enum Language {
    KOTLIN,
    PYTHON,
    SWIFT,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Language::KOTLIN => write!(f, "kotlin"),
            Language::SWIFT => write!(f, "swift"),
            Language::PYTHON => write!(f, "python"),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    UnsupportedLanguage,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for Language {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "kotlin" => Ok(Language::KOTLIN),
            "python" => Ok(Language::PYTHON),
            "swift" => Ok(Language::SWIFT),
            _ => Err(Error::UnsupportedLanguage),
        }
    }
}

fn generate_bindings(opt: &Opt) -> anyhow::Result<(), anyhow::Error> {
    uniffi_bindgen::generate_bindings(
        &opt.udl_file,
        None,
        vec![opt.language.to_string().as_str()],
        Some(&opt.out_dir),
        false,
    )?;

    Ok(())
}

fn fixup_python_lib_path(
    out_dir: &PathBuf,
    lib_name: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::io::Write;

    const LOAD_INDIRECT_DEF: &str = "def loadIndirect():";

    let bindings_file = out_dir.join("bdk.py");
    let mut data = fs::read_to_string(&bindings_file)?;

    let pos = data.find(LOAD_INDIRECT_DEF).expect(&format!(
        "loadIndirect not found in `{}`",
        bindings_file.display()
    ));
    let range = pos..pos + LOAD_INDIRECT_DEF.len();

    let replacement = format!(
        r#"
def loadIndirect():
    import glob
    return getattr(ctypes.cdll, glob.glob(os.path.join(os.path.dirname(os.path.abspath(__file__)), '{}.*'))[0])

def _loadIndirectOld():"#,
        &lib_name.to_str().expect("lib name")
    );
    data.replace_range(range, &replacement);

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&bindings_file)?;
    file.write(data.as_bytes())?;

    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "bdk-ffi-bindgen",
    about = "A tool to generate bdk-ffi language bindings"
)]
struct Opt {
    /// UDL file
    #[structopt(env = "BDKFFI_BINDGEN_UDL", short, long, default_value("src/bdk.udl"), parse(try_from_str = PathBuf::from_str))]
    udl_file: PathBuf,

    /// Language to generate bindings for
    #[structopt(env = "BDKFFI_BINDGEN_LANGUAGE", short, long, possible_values(&["kotlin","swift","python"]), parse(try_from_str = Language::from_str))]
    language: Language,

    /// Output directory to put generated language bindings
    #[structopt(env = "BDKFFI_BINDGEN_OUTPUT_DIR", short, long, parse(try_from_str = PathBuf::from_str))]
    out_dir: PathBuf,

    /// Python fix up lib path
    #[structopt(env = "BDKFFI_BINDGEN_PYTHON_FIXUP_PATH", short, long, parse(try_from_str = PathBuf::from_str))]
    python_fixup_path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    println!("Input UDL file is {:?}", opt.udl_file);
    println!("Chosen language is {}", opt.language);
    println!("Output directory is {:?}", opt.out_dir);

    generate_bindings(&opt)?;

    if opt.language == Language::PYTHON {
        if let Some(path) = opt.python_fixup_path {
            println!("Fixing up python lib path, {:?}", &path);
            fixup_python_lib_path(&opt.out_dir, &path)?;
        }
    }
    Ok(())
}
