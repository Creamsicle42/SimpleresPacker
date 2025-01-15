use std::{cmp::min, io::Write, u16, usize};
#[allow(unused_variables, unused_mut)]
use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read},
};

struct LZ77Codeword {
    lookback: u16,
    length: u8,
    token: u8,
}

impl LZ77Codeword {
    fn to_le_bytes(&self) -> [u8; 4] {
        let mut out: [u8; 4] = [0; 4];
        let lb = self.lookback.to_le_bytes();
        out[0] = lb[0];
        out[1] = lb[1];
        let ln = self.length.to_le_bytes();
        let tk = self.token.to_le_bytes();
        out[2] = ln[0];
        out[3] = tk[0];
        out
    }
}

pub fn buffer_compress(
    mut reader: BufReader<File>,
    mut writer: BufWriter<File>,
) -> Result<(), io::Error> {
    let mut bytes_vec: Vec<u8> = vec![];
    let byte_count = reader.read_to_end(&mut bytes_vec)?;
    let raw_bytes = bytes_vec.as_slice();
    let mut byte_on: usize = 0;
    while byte_on < byte_count {
        let next_codeword = get_best_codeword(&raw_bytes, &byte_on.into());
        byte_on += usize::from(next_codeword.length + 1);
        let _ = writer.write(&next_codeword.to_le_bytes());
    }
    let _ = writer.flush();
    return Ok(());
}

fn get_best_codeword(bytes: &[u8], position: &usize) -> LZ77Codeword {
    // Determine size of lookahead and lookback buffers (the last byte of the file should be the
    // last literal value)
    let p: u64 = *position as u64;
    let lookback_buf_size: u16 = min(u64::from(u16::MAX), p).try_into().unwrap();
    let lookahead_buf_size: u8 = u8::try_from(min(255_u64, bytes.len() as u64 - p)).unwrap() - 1_u8;
    // Initialize run size to zero
    let mut run_size: u8 = 0;
    // Initialize vec of all possible lookback values
    let mut lookback_values: Vec<u16> = (0..lookback_buf_size).into_iter().collect();
    // Keep track of nearest lookback
    let mut best_lookback: u16 = 0;
    // While the lookback list is not empty and run size is less than max...
    while !lookback_values.is_empty() && run_size < lookahead_buf_size {
        // - Filter lookback vec down to values that have are valid lookbacks for size of run + 1
        lookback_values = lookback_values
            .into_iter()
            .filter(|lb| {
                bytes[lb.clone() as usize + run_size as usize]
                    == bytes[position + run_size as usize]
            })
            .collect();
        // - If vec is not empty then update best run size and closest lookback
        if !lookback_values.is_empty() {
            best_lookback = lookback_values.first().unwrap().clone();
            run_size += 1;
        }
    }
    // After this we know know the position of the best lookback and the run length
    LZ77Codeword {
        length: run_size,
        lookback: best_lookback,
        token: bytes[position + usize::from(run_size)],
    }
}
