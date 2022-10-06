use std::{
    fs::{self, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use once_cell::sync::Lazy;
use structopt::StructOpt;

static OPTS: Lazy<CliOpts> = Lazy::new(CliOpts::from_args);

#[derive(Clone, Debug, StructOpt)]
enum CliOpts {
    Merge(MergeOptions),
    Split(SplitOptions),
}

#[derive(Clone, Debug, StructOpt)]
struct MergeOptions {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    /// Output file base name, output.dat if not present
    ///
    /// Will be appended the sequence number.
    #[structopt(parse(from_os_str), default_value = "output.dat")]
    output: PathBuf,
}

#[derive(Clone, Debug, StructOpt)]
struct SplitOptions {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    /// Output file base name, output.spl if not present
    ///
    /// Will be appended the sequence number.
    #[structopt(parse(from_os_str), default_value = "output.spl")]
    output: PathBuf,

    /// Max size in bytes per split file
    /// Default value: 1 MiB
    #[structopt(short, long, default_value = "1048576")]
    size: u64,
}

pub fn main() -> Result<()> {
    match &*OPTS {
        CliOpts::Merge(o) => merge_files(o),
        CliOpts::Split(o) => split_file(o),
    }
}

fn merge_files(options: &MergeOptions) -> Result<()> {
    let input_path = &options.input;
    let output_path = &options.output;

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

    let mut output_file = BufWriter::new(write_options.open(output_path)?);

    let mut files = fs::read_dir(".")?
        .flatten()
        .filter(|f| {
            f.file_name()
                .to_string_lossy()
                .starts_with(&*input_path.file_name().unwrap().to_string_lossy())
        })
        .collect::<Vec<_>>();
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

    for file in files {
        //        println!("{}", file.path().display());
        let mut input_file = BufReader::new(read_options.open(file.path())?);
        io::copy(&mut input_file, &mut output_file)?;
    }

    Ok(())
}

fn split_file(options: &SplitOptions) -> Result<()> {
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
    let mut input_file = BufReader::new(read_options.open(&options.input)?);
    let output_path = &options.output;
    let len = input_file.get_ref().metadata()?.len();
    if len <= options.size {
        let mut output_file = write_options.open(output_path)?;
        io::copy(&mut input_file, &mut output_file)?;
    } else {
        let mut written = 0;
        let mut chunk = 0;
        let output_path = {
            let ext = {
                let mut p = output_path
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
            output_path.with_extension(ext)
        };
        while written < len {
            let mut output_file =
                BufWriter::new(write_options.open(output_path.with_extension(format!("{chunk}")))?);
            written += io::copy(
                &mut input_file.by_ref().take(options.size),
                &mut output_file,
            )?;
            output_file.flush()?;
            output_file.get_ref().sync_all()?;
            chunk += 1;
        }
    }
    Ok(())
}
