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

#[cfg(test)]
mod tests {
    use super::*;

    mod merge {
        use std::io::Cursor;

        use super::*;

        #[test]
        fn empty() {
            let input = Vec::<Result<Cursor<Vec<u8>>, _>>::new();
            let mut output = Cursor::new(vec![]);

            merge_files(input, &mut output).unwrap();

            assert_eq!(output.into_inner(), vec![]);
        }

        #[test]
        fn one() {
            let input = vec![Ok(Cursor::new(&[10_u8][..]))];
            let mut output = Cursor::new(vec![]);

            merge_files(input, &mut output).unwrap();

            assert_eq!(output.into_inner(), vec![10]);
        }

        #[test]
        fn two() {
            let input = vec![
                Ok(Cursor::new(&[1_u8][..])),
                Ok(Cursor::new(&[2_u8, 3_u8][..])),
            ];
            let mut output = Cursor::new(vec![]);

            merge_files(input, &mut output).unwrap();

            assert_eq!(output.into_inner(), vec![1, 2, 3]);
        }
    }

    mod split {
        use std::{cell::RefCell, io::Cursor, rc::Rc};

        use super::*;

        #[derive(Debug, Default, Clone)]
        struct Z(Rc<RefCell<Cursor<Vec<u8>>>>);

        impl Z {
            pub fn into_inner(self) -> Vec<u8> {
                Rc::try_unwrap(self.0).unwrap().into_inner().into_inner()
            }
        }

        impl Write for Z {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.borrow_mut().write(buf)
            }

            fn flush(&mut self) -> std::io::Result<()> {
                self.0.borrow_mut().flush()
            }
        }

        impl Seek for Z {
            fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
                self.0.borrow_mut().seek(pos)
            }
        }

        #[test]
        fn none() {
            let mut out = vec![];
            let f = || {
                out.push(Z::default());
                Ok(out.last().unwrap().clone())
            };
            let input = [];
            split_file(100, Cursor::new(input), f).unwrap();
            assert_eq!(
                out.into_iter().flat_map(Z::into_inner).collect::<Vec<_>>(),
                vec![]
            );
        }

        #[test]
        fn one() {
            let mut out = vec![];
            let f = || {
                out.push(Z::default());
                Ok(out.last().unwrap().clone())
            };
            let input = [1, 2, 3, 4];
            split_file(100, Cursor::new(input), f).unwrap();
            let out = out.into_iter().map(Z::into_inner).collect::<Vec<_>>();
            assert_eq!(out.len(), 1);
            assert_eq!(out, [vec![1, 2, 3, 4]]);
        }
        #[test]
        fn two() {
            let mut out = vec![];
            let f = || {
                out.push(Z::default());
                Ok(out.last().unwrap().clone())
            };
            let input = [1, 2, 3, 4];
            split_file(2, Cursor::new(input), f).unwrap();
            let out = out.into_iter().map(Z::into_inner).collect::<Vec<_>>();
            assert_eq!(out.len(), 2);
            assert_eq!(out, [vec![1, 2], vec![3, 4]]);
        }

        #[test]
        fn uneven() {
            let mut out = vec![];
            let f = || {
                out.push(Z::default());
                Ok(out.last().unwrap().clone())
            };
            let input = [1, 2, 3, 4, 5];
            split_file(2, Cursor::new(input), f).unwrap();
            let out = out.into_iter().map(Z::into_inner).collect::<Vec<_>>();
            assert_eq!(out.len(), 3);
            assert_eq!(out, [vec![1, 2], vec![3, 4], vec![5]]);
        }
    }
}
