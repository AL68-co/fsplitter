use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Args, Parser};
use fsplitter::{merge_files, split_file};

#[derive(Clone, Debug, Parser)]
enum CliOpts {
    Merge(MergeOptions),
    Split(SplitOptions),
}

#[derive(Clone, Debug, Args)]
struct MergeOptions {
    /// Input file
    #[arg()]
    input: PathBuf,
    /// Output file base name, output.dat if not present
    ///
    /// Will be appended the sequence number.
    #[arg(default_value = "output.dat")]
    output: PathBuf,
}

#[derive(Clone, Debug, Args)]
struct SplitOptions {
    /// Input file
    #[arg()]
    input: PathBuf,
    /// Output file base name, output.spl if not present
    ///
    /// Will be appended the sequence number.
    #[arg(default_value = "output.spl")]
    output: PathBuf,

    /// Max size in bytes per split file
    /// Default value: 1 MiB
    #[arg(short, long, default_value = "1048576")]
    size: u64,
}

fn main() -> Result<()> {
    let read_options = {
        let mut o = OpenOptions::new();
        o.read(true).create(false);
        o
    };
    let write_options = {
        let mut o = OpenOptions::new();
        o.write(true).create(true).truncate(true);
        o
    };
    match CliOpts::parse() {
        CliOpts::Merge(o) => {
            let mut files = fs::read_dir(".")?
                .filter(|f| match f {
                    Ok(f) => f
                        .file_name()
                        .to_string_lossy()
                        .starts_with(&*o.input.file_name().unwrap().to_string_lossy()),
                    Err(_) => true,
                })
                .collect::<Result<Vec<_>, _>>()?;
            files.sort_by_cached_key(|v| {
                PathBuf::from(v.file_name())
                    .extension()
                    .and_then(|e| {
                        e.to_str().map(|s| {
                            s.parse::<u64>()
                                .unwrap_or_else(|e| panic!("Invalid input file {s}: {e}"))
                        })
                    })
                    .unwrap_or(0u64)
            });
            merge_files(
                files.into_iter().map(|p| read_options.open(p.path())),
                write_options.open(o.output)?,
            )
        }
        CliOpts::Split(o) => {
            let input_file = read_options.open(&o.input)?;

            let output_maker = {
                let mut chunk = 0;
                let output_path = {
                    let ext = {
                        let mut p = o
                            .output
                            .extension()
                            .map(|v| {
                                let mut s = v.to_owned();
                                s.push(".");
                                s
                            })
                            .unwrap_or_else(|| "".into());
                        p.push("ext");
                        p
                    };
                    o.output.with_extension(ext)
                };
                move || {
                    let f = write_options.open(output_path.with_extension(format!("{chunk}")))?;
                    chunk += 1;
                    Ok(f)
                }
            };

            split_file(o.size, input_file, output_maker)
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn clap() {
        CliOpts::command().debug_assert();
    }
}
