#![allow(dead_code)]
use console::style;
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};

mod lz77;

const FLAG_LZ77_COMPRESSED: u16 = 1;

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

struct ResData {
    res: Resource,
    id_off: u32,
    id_len: u16,
    dat_off: u32,
    dat_len: u32,
    uncompressed_len: u32,
}

#[derive(Debug)]
pub enum PackError {
    FilesystemError(io::Error),
    ParseError(serde_yaml::Error),
    MissingBaseFile(PathBuf),
}

impl Resource {
    // Get an owned relative path to the resource file
    pub fn get_file_path(&self) -> PathBuf {
        PathBuf::from(self.filepath.as_str())
    }
    // Get an owned relative path to the data file
    pub fn get_data_file_path(&self) -> PathBuf {
        let mut out = PathBuf::from(self.filepath.as_str());
        out.set_extension("bin");
        return out;
    }
}

impl From<Resource> for ResData {
    fn from(value: Resource) -> Self {
        ResData {
            id_len: value.id.len().try_into().unwrap(),
            res: value,
            id_off: 0,
            dat_len: 0,
            dat_off: 0,
            uncompressed_len: 0,
        }
    }
}

pub fn write_resource_file(
    manifest_file_path: PathBuf,
    pack_file_path: PathBuf,
) -> Result<(), PackError> {
    println!(
        "Opening manifest file {}.",
        style(manifest_file_path.canonicalize().unwrap().display()).magenta()
    );

    // Open and parse manifest file
    let manifest_file = match File::open(&manifest_file_path) {
        Ok(f) => f,
        Err(e) => {
            println!("{}", style("Error in reading manifest file.").red());
            return Err(PackError::FilesystemError(e));
        }
    };

    let manifest: Vec<Resource> = match serde_yaml::from_reader(manifest_file) {
        Ok(t) => t,
        Err(e) => {
            println!("{}", style("Error in parsing manifest file.").red());
            return Err(PackError::ParseError(e));
        }
    };

    println!("Checking resource files...");
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

    println!("Generating binary files.");
    for res in files_to_remake {
        let f_path = manifest_file_path.parent().unwrap().join(&res.filepath);
        let _ = generate_bin_file(&f_path, &res.compression);
    }

    println!("Generating ID section.");
    // Compile id section and offsets
    let mut id_section = String::from_utf8([].to_vec()).unwrap();

    let mut resources: Vec<ResData> = manifest.into_iter().map(|res| ResData::from(res)).collect();

    for res in resources.iter_mut() {
        res.id_off = id_section.len().try_into().unwrap();
        id_section.push_str(res.res.id.as_str());
        res.uncompressed_len =
            fs::metadata(manifest_file_path.parent().unwrap().join(&res.res.filepath))
                .unwrap()
                .len()
                .try_into()
                .unwrap();
        res.dat_len = fs::metadata(
            manifest_file_path
                .parent()
                .unwrap()
                .join(&res.res.filepath)
                .with_extension("bin"),
        )
        .unwrap()
        .len()
        .try_into()
        .unwrap();
    }

    let mut id_section_length: u32 = id_section.len().try_into().unwrap();
    for _ in 0..id_section_length % 4 {
        id_section.push('\0');
        id_section_length += 1;
    }

    println!("Creating resource pack file...");
    // Open resource file
    let r_file = match File::create(pack_file_path) {
        Ok(f) => f,
        Err(e) => {
            println!(
                "{}",
                style("ERROR: Could not open or create resource pack file.").red()
            );
            return Err(PackError::FilesystemError(e));
        }
    };

    println!("Writing file header.");
    let mut file_writer = BufWriter::new(r_file);

    // Write file header
    let _ = file_writer.write(b"smpr"); // Intro bytes
    let _ = file_writer.write(&1_u16.to_be_bytes()); // File version
    let len: u16 = resources.len().try_into().unwrap();
    let _ = file_writer.write(&len.to_be_bytes()); // Resource count

    // Write resource id section and keep track of slices
    let _ = file_writer.write(&id_section_length.to_be_bytes());
    let _ = file_writer.write(id_section.as_bytes());

    // Write out header section
    let res_count: u32 = resources.len().try_into().unwrap();
    let data_section_start: u32 = 12 + id_section_length + (16 * res_count);
    let mut data_written: u32 = 0;
    for res in resources.iter() {
        let _ = file_writer.write(&res.id_off.to_be_bytes());
        let _ = file_writer.write(&res.id_len.to_be_bytes());
        let mut flags: u16 = 0;
        if res.res.compression == CompressionType::LZ77 {
            flags &= FLAG_LZ77_COMPRESSED;
        }

        let _ = file_writer.write(&flags.to_be_bytes());
        let d_start: u32 = data_section_start + data_written;
        data_written += res.dat_len;
        let _ = file_writer.write(&d_start.to_be_bytes());
        let _ = file_writer.write(&res.dat_len.to_be_bytes());
        let _ = file_writer.write(&res.uncompressed_len.to_be_bytes());
    }

    // Copy down binary section
    for res in resources.iter() {
        println!(
            "Copying file resource {}...",
            style(res.res.get_data_file_path().display()).magenta()
        );
        let mut bin_file = File::open(
            manifest_file_path
                .parent()
                .unwrap()
                .join(&res.res.get_data_file_path()),
        )
        .unwrap();
        let _ = io::copy(&mut bin_file, &mut file_writer);
    }

    println!("Finalizing resource pack.");
    let _ = file_writer.flush();

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
            println!(
                "Creating uncompressed binary for {}.",
                style(path.display()).magenta()
            );
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
    println!("Checking file {}...", style(file.display()).magenta());
    let base_metadata = fs::metadata(file);
    let mut bin_path = file.clone();
    bin_path.set_extension(".bin");
    let bin_metadata = fs::metadata(bin_path);
    if !base_metadata.is_ok() {
        println!(
            "{}",
            style("WARNING: Base resource file is missing, resource will be skipped...").red()
        );
        return FileCheckResult::BaseMissing;
    }
    if !bin_metadata.is_ok() {
        println!("{}", style("Binary file missing, will be built.").yellow());
        return FileCheckResult::BinMissing;
    }
    if bin_metadata.unwrap().mtime() < base_metadata.unwrap().mtime() {
        println!(
            "{}",
            style("Binary file out of date, will be rebuilt.").yellow()
        );
        return FileCheckResult::BinOutOfDate;
    }
    println!("{}", style("Binary file is okay.").green());
    return FileCheckResult::FileOkay;
}
