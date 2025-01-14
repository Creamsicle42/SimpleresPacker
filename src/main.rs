use clap::Parser;
use std::{path::PathBuf, str::FromStr};

mod packer;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CLIArgs {
    #[arg(short, long)]
    manifest: Option<PathBuf>,
    pack: Option<PathBuf>,
}

fn main() {
    let args = CLIArgs::parse();
    let err = packer::write_resource_file(
        args.manifest
            .unwrap_or(PathBuf::from_str(".manifest.yaml").expect(""))
            .clone(),
        args.pack
            .unwrap_or(PathBuf::from_str("pack.smr").expect("")),
    );

    if let Err(e) = err {
        println!("Resource pack creating failed with error {:?}.", e);
    } else {
        println!("Resource pack succesfully created.");
    }
}
