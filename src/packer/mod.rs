#![allow(dead_code)]
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};

// Compression types
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub enum CompressionType {
    NONE, // Compression resource
    LZ77, // LZ77 encryption
}

// Resource struct type
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Resource {
    pub id: String,                   // The resource ID
    pub compression: CompressionType, // The resource compression type
    pub filepath: String,             // Relative filepath to the base file
}

pub enum PackError {
    FilesystemError(io::Error),
    ParseError(serde_yaml::Error),
    MissingBaseFile(PathBuf),
}

impl Resource {
    // Get an owned relative path to the resource file
    pub fn get_file_path(self) -> PathBuf {
        PathBuf::from(self.filepath.as_str())
    }
    // Get an owned relative path to the data file
    pub fn get_data_file_path(self) -> PathBuf {
        let mut out = PathBuf::from(self.filepath.as_str());
        out.set_extension("bin");
        return out;
    }
}

pub fn write_resource_file(
    manifest_file_path: PathBuf,
    pack_file_path: PathBuf,
) -> Result<(), PackError> {
    // Open and parse manifest file
    let manifest_file = match File::open(&manifest_file_path) {
        Ok(f) => f,
        Err(e) => {
            return Err(PackError::FilesystemError(e));
        }
    };

    let manifest: Vec<Resource> = match serde_yaml::from_reader(manifest_file) {
        Ok(t) => t,
        Err(e) => {
            return Err(PackError::ParseError(e));
        }
    };

    // Make sure all resource binaries are up to date and get their data
    let files_to_remake: Vec<&Resource> = manifest
        .iter()
        .filter(|&res| {
            let path = manifest_file_path
                .clone()
                .parent()
                .unwrap()
                .join(&res.filepath);
            return match check_file(&path) {
                FileCheckResult::FileOkay => false,
                FileCheckResult::BinMissing => true,
                FileCheckResult::BinOutOfDate => true,
                FileCheckResult::BaseMissing => false,
            };
        })
        .collect();

    for res in files_to_remake {
        let f_path = manifest_file_path.parent().unwrap().join(&res.filepath);
        generate_bin_file(&f_path, &res.compression);
    }

    // Open resource file
    // Write file header
    // Write resource id section and keep track of slices
    // Write out header section
    // Copy down binary section
    Ok(())
}

enum FileCheckResult {
    FileOkay,
    BinOutOfDate,
    BinMissing,
    BaseMissing,
}

fn generate_bin_file(path: &PathBuf, comp: &CompressionType) -> io::Result<()> {
    let base_file = File::open(path)?;
    let bin_file = File::create(path.clone().with_extension("bin"))?;
    match comp {
        CompressionType::NONE => {
            write_bin_file_uncompressed(BufReader::new(base_file), BufWriter::new(bin_file));
        }
        CompressionType::LZ77 => todo!(),
    };
    return Ok(());
}

fn write_bin_file_uncompressed(mut reader: BufReader<File>, mut writer: BufWriter<File>) {
    let _ = io::copy(&mut reader, &mut writer);
}

// Check if a file needs to be regenerated
fn check_file(file: &PathBuf) -> FileCheckResult {
    let base_metadata = fs::metadata(file);
    let mut bin_path = file.clone();
    bin_path.set_extension(".bin");
    let bin_metadata = fs::metadata(bin_path);
    if !base_metadata.is_ok() {
        return FileCheckResult::BaseMissing;
    }
    if !bin_metadata.is_ok() {
        return FileCheckResult::BinMissing;
    }
    if bin_metadata.unwrap().mtime() < base_metadata.unwrap().mtime() {
        return FileCheckResult::BinOutOfDate;
    }
    return FileCheckResult::FileOkay;
}
