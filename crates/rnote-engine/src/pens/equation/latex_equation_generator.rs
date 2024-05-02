use std::{io::Write, path::Path, process::Command};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone)]
pub struct Environment {
    pub pre_preamble: String,
    pub pre_code: String,
    pub post_code: String,
}

#[derive(Debug, Clone)]
pub struct LatexContext {
    pub preamble: String,
    pub environment: Environment,
}

impl LatexContext {
    fn preprocess_code(&self, latex_code: &String) -> String {
        let mut complete_code = self.environment.pre_preamble.clone();
        complete_code.push_str(&self.preamble);
        complete_code.push_str(self.environment.pre_code.as_str());
        complete_code.push_str(&latex_code);
        complete_code.push_str(self.environment.post_code.as_str());

        complete_code
    }
}

fn create_command(name: &str) -> Command {
    let mut command = Command::new(name);

    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: i32 = 0x08000000;
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    command
}

pub fn create_svg_from_latex(
    latex_code: &String,
    context: &LatexContext,
) -> Result<String, String> {
    // Create temporary directory
    let mut tmpdir = tempfile::tempdir().unwrap();

    // Preprocess latex code
    let preprocessed_code = context.preprocess_code(latex_code);

    // Open file
    let tex_path = tmpdir.path().join(Path::new("ezlatex.tex"));
    let dvi_path = tmpdir.path().join(Path::new("ezlatex.dvi"));
    let svg_path = tmpdir.path().join(Path::new("ezlatex.svg"));
    let mut file = std::fs::File::create(&tex_path).unwrap();
    // Write to file
    file.write_all(preprocessed_code.as_bytes()).unwrap();
    // Close file
    drop(file);

    // Compile and convert to DVI

    // TODO: Wrap command calls in fn
    let output_latex = create_command("latex")
        .current_dir(tmpdir.path())
        .arg(&tex_path)
        .output()
        .unwrap();

    if !output_latex.status.success() {
        // Stdout might contain non-UTF8 characters due to latex sometimes having issues when
        // creating a command at the beginning of the file which starts with some special characters,
        // so this is needed
        return Result::Err(String::from_utf8_lossy(output_latex.stdout.as_slice()).to_string());
    }

    // ezlatex.dvi will have been created
    let output_svg = create_command("dvisvgm")
        .current_dir(tmpdir.path())
        .arg("-n")
        .arg(&dvi_path)
        .output()
        .unwrap();

    if !output_svg.status.success() {
        return Result::Err(String::from_utf8_lossy(output_latex.stderr.as_slice()).to_string());
    }

    // ezlatex.svg will have been created

    Result::Ok(std::fs::read_to_string(&svg_path).unwrap())
}
