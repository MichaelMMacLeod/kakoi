// This code comes from a tutorial on Wgpu, though some modifications were made
// to it.
//
// Source: https://github.com/sotrh/learn-wgpu
//
// MIT License
//
// Copyright (c) 2020 Benjamin Hansen
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use anyhow::*;
use glob::glob;
use rayon::prelude::*;
use std::{
    env,
    fs::{read_to_string, write},
    path::PathBuf,
};

const SHADER_SRC_DIR: &str = "src/shaders/src";
const SHADER_BUILD_DIR: &str = "src/shaders/build";

struct ShaderData {
    src: String,
    src_path: PathBuf,
    spv_path: PathBuf,
    kind: shaderc::ShaderKind,
}

impl ShaderData {
    pub fn load(src_path: PathBuf) -> Result<Self> {
        let extension = src_path
            .extension()
            .context("File has no extension")?
            .to_str()
            .context("Extension cannot be converted to &str")?;
        let kind = match extension {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => bail!("Unsupported shader: {}", src_path.display()),
        };

        let build_dir: PathBuf = SHADER_BUILD_DIR.into();

        let src = read_to_string(src_path.clone())?;
        let spv_path = build_dir
            .join(src_path.strip_prefix(SHADER_SRC_DIR)?)
            .with_extension(format!("{}.spv", extension));

        Ok(Self {
            src,
            src_path,
            spv_path,
            kind,
        })
    }
}

fn main() -> Result<()> {
    let mut shader_paths = Vec::new();

    for kind in &["vert", "frag", "comp"] {
        shader_paths.extend(glob(&*format!("{}/**/*.{}", SHADER_SRC_DIR, kind))?);
    }

    let shaders = shader_paths
        .into_par_iter()
        .map(|glob_result| ShaderData::load(glob_result?))
        .collect::<Result<Vec<_>>>()?;

    let mut compiler = shaderc::Compiler::new().context("Unable to create shader compiler")?;

    for shader in shaders {
        println!(
            "cargo:rerun-if-changed={}",
            shader.src_path.as_os_str().to_str().unwrap()
        );

        let compiled = compiler.compile_into_spirv(
            &shader.src,
            shader.kind,
            &shader.src_path.to_str().unwrap(),
            "main",
            None,
        )?;
        write(shader.spv_path, compiled.as_binary_u8())?;
    }

    Ok(())
}
