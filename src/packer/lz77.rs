use std::{cmp::min, io::Write, u16, u8, usize};
#[allow(unused_variables, unused_mut)]
use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read},
};

// Encodes an LZ77 codeword
enum LZ77Codeword {
    Literal(u8),  // A literal, converted to a null byte followed by the literal
    Run(u16, u8), // A codeword, represented by a two byte lookback and a one byte run length
}

impl LZ77Codeword {
    fn write_to_buffer(&self, buff: &mut BufWriter<File>) {
        match self {
            LZ77Codeword::Literal(ch) => {
                let _ = buff.write(&ch.to_le_bytes());
                let _ = buff.write(&['\0' as u8]);
            }
            LZ77Codeword::Run(lookback, len) => {
                let _ = buff.write(&(lookback + 256).to_le_bytes());
                let _ = buff.write(&len.to_le_bytes());
            }
        }
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
    let mut codewords: Vec<LZ77Codeword> = vec![];
    while byte_on < byte_count {
        let new_codeword = get_best_codeword(&raw_bytes, &byte_on);
        if let LZ77Codeword::Run(_, len) = new_codeword {
            byte_on += len as usize;
        } else {
            byte_on += 1_usize;
        }
        codewords.push(new_codeword);
    }
    for codeword in codewords {
        codeword.write_to_buffer(&mut writer);
    }
    let _ = writer.flush();
    return Ok(());
}

// Takes in a byte buffer and a position in that buffer, returns the best possible codeword for
// that position.
fn get_best_codeword(bytes: &[u8], position: &usize) -> LZ77Codeword {
    // Special case 1, if we are in the last 3 bytes of the file, then these will be literals
    if bytes.len() - position < 3 {
        return LZ77Codeword::Literal(bytes[*position]);
    }
    // Special case 2, The first byte in the file must always be a literal
    if *position == 0_usize {
        return LZ77Codeword::Literal(bytes[*position]);
    }
    // Final case, we can try to find runs
    // Get a sample slice of the next three bytes after pos
    let ref_slice = &bytes[*position..*position + 3_usize];
    let max_lookback: usize = min(*position, (u16::MAX - u8::MAX as u16) as usize);
    let mut valid_lookbacks: Vec<usize> = (1_usize..max_lookback)
        .into_iter()
        .filter(|lb| slice_compare(&bytes[(*position - lb)..((*position - lb) + 3)], ref_slice))
        .collect();

    // If there are no runs in the lookback section worth considering then return a literal
    if valid_lookbacks.is_empty() {
        return LZ77Codeword::Literal(bytes[*position]);
    }

    // Repeatedly filter lookback positions by longer run lengths
    let mut best_lookback: usize = valid_lookbacks.first().unwrap().clone();
    let mut best_run = 3_usize;
    let max_run = min(u8::MAX as usize, bytes.len() - *position);

    while !valid_lookbacks.is_empty() && best_run < max_run {
        valid_lookbacks = valid_lookbacks
            .into_iter()
            .filter(|lb| bytes[position + best_run] == bytes[(position - lb) + best_run])
            .collect();
        if let Some(lb) = valid_lookbacks.first() {
            best_run += 1;
            best_lookback = *lb;
        }
    }

    return LZ77Codeword::Run(
        best_lookback.try_into().unwrap(),
        best_run.try_into().unwrap(),
    );
}

fn slice_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        if a[i] != b[i] {
            return false;
        }
    }
    return true;
}
