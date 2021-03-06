use sass_rs::{compile_file, Options, OutputStyle};

use crate::utils::AnyResult;

pub mod data;
pub mod html;
pub mod status;

pub fn init() -> AnyResult {
    let style_css = compile_file(
        "./assets/main.sass",
        Options {
            output_style: OutputStyle::Compressed,
            ..Default::default()
        },
    )
    .expect("failed to compile style sheet");
    std::fs::write("./static/style.css", style_css).expect("failed to write style.css");

    Ok(())
}
