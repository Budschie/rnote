use std::{io::Write, path::Path, process::Command};

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
    let output_latex = Command::new("latex")
        .current_dir(tmpdir.path())
        .arg(&tex_path)
        .output()
        .unwrap();

    if !output_latex.status.success() {
        return Result::Err(String::from_utf8(output_latex.stdout).unwrap());
    }

    // ezlatex.dvi will have been created
    let output_svg = Command::new("dvisvgm")
        .current_dir(tmpdir.path())
        .arg("-n")
        .arg(&dvi_path)
        .output()
        .unwrap();

    if !output_svg.status.success() {
        return Result::Err(String::from_utf8(output_latex.stderr).unwrap());
    }

    // ezlatex.svg will have been created

    Result::Ok(std::fs::read_to_string(&svg_path).unwrap())
}
