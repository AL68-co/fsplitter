use std::io::{self, BufReader, BufWriter, Read, Seek, Write};

use anyhow::Result;

pub fn merge_files(
    input_files: impl IntoIterator<Item = Result<impl Read + Seek, io::Error>>,
    output: impl Write + Seek,
) -> Result<()> {
    let mut output_file = BufWriter::new(output);

    for file in input_files {
        //        println!("{}", file.path().display());
        let mut input_file = BufReader::new(file?);
        io::copy(&mut input_file, &mut output_file)?;
    }

    Ok(())
}

pub fn split_file<O: Write + Seek>(
    max_size: u64,
    input_file: impl Read + Seek,
    mut outputs: impl FnMut() -> Result<O, io::Error>,
) -> Result<()> {
    let mut input_file = BufReader::new(input_file);
    let start = input_file.stream_position()?;
    let end = input_file.seek(io::SeekFrom::End(0))?;
    let len = end - start;
    input_file.seek(io::SeekFrom::Start(start))?;
    if len <= max_size {
        let mut output_file = ((outputs)())?;
        io::copy(&mut input_file, &mut output_file)?;
    } else {
        let mut written = 0;
        while written < len {
            let mut output_file = BufWriter::new((outputs)()?);
            written += io::copy(&mut input_file.by_ref().take(max_size), &mut output_file)?;
            output_file.flush()?;
        }
    }
    Ok(())
}
