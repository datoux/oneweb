use clap::Parser;
use std::fs;

mod clustering;
mod data_processor;
mod gps_processor;
mod info_processor;
mod processor;
mod tpx3lut;
mod utils;

/// Convertor of oneweb timepix data
#[derive(Parser, Debug)]
#[command(version = "1.0", about = "Convertor of oneweb timepix data")]
struct Cli {
    /// Path to gps file (dosimeter_gps_info.csv)
    #[arg(short = 'g', long)]
    gps_file: String,

    /// Path to measurement file (dosimeter_measure_info.csv)
    #[arg(short = 'm', long)]
    meas_file: String,

    /// Path to data file (dosimeter_image_packets.csv)
    #[arg(short = 'd', long)]
    data_file: String,

    /// Output directory
    #[arg(short = 'o', long)]
    output_directory: String,
}

fn main() {
    let args = Cli::parse();

    let mut processor = processor::Processor::new();
    let gps_file = args.gps_file;
    let meas_file = args.meas_file;
    let data_file = args.data_file;
    let out_dir = args.output_directory;

    if fs::create_dir_all(&out_dir).is_err() {
        eprintln!("Error creating output directory: {}", out_dir);
        return;
    }

    if let Err(e) = processor.process_files(&gps_file, &meas_file, &data_file, &out_dir) {
        let error_message = e.to_string();
        if error_message.contains("No more data available") {
            println!("Done.");
            return;
        }
        eprintln!("Error processing files: {:?}", e);
    }
}
