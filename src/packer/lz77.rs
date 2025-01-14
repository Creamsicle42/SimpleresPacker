use std::io::Write;
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
    todo!();
}
