#![allow(dead_code)]
use std::path::PathBuf;

// Encryption types
pub enum EncryptionType {
    NONE, // Unencrypted resource
    LZ77, // LZ77 encryption
}

// Resource struct type
pub struct Resource {
    pub id: String,                 // The resource ID
    pub encryption: EncryptionType, // The resource encryption type
    pub filepath: String,           // Relative filepath to the base file
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

pub fn write_resource_file(manifest_file: PathBuf) {
    // Open and parse manifest file
    // Make sure all resource binaries are up to date and get their data
    // Open resource file
    // Write file header
    // Write resource id section and keep track of slices
    // Write out header section
    // Copy down binary section
}
