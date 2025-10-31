use core::panic;
use std::env;
use std::collections::HashSet;
use std::io::{BufWriter, Write};
use std::fs::File;
use flate2::write::GzEncoder;
use flate2::Compression;
use rayon::prelude::*; // For parallel processing

/// The main function parses command-line arguments, processes the input CSV file,
/// optionally uses a zones CSV file, and writes the output in MTX format.
fn main() {
    let arg: Vec<String> = env::args().collect();

    if arg.len() < 3 {
        println!("Usage: csv_to_mtx <input.csv> <output.mtx/.mtx.gz> [zones.csv]");
        return;
    }

    let data = read_csv(&arg[1]);
    let all_zones = get_all_zones(&arg, &data);
    println!("Found {} zones", all_zones.len());
    let matrix = build_matrix(&data, &all_zones);
    write_mtx_file(&arg[2], &all_zones, &matrix);
}

/// Reads the input CSV file and extracts the data as a vector of tuples containing
/// origin, destination, and value. Automatically detects the CSV format:
/// - 3-column format: origin, destination, value
/// - Rectangular format: first row contains destinations, first column contains origins
///
/// # Arguments
/// * `input_file` - The path to the input CSV file.
///
/// # Returns
/// A vector of tuples `(i32, i32, f32)` representing the origin, destination, and value.
fn read_csv(input_file: &str) -> Vec<(i32, i32, f32)> {
    let file = match File::open(input_file) {
        Ok(f) => f,
        Err(e) => {
            panic!("Error opening file {input_file}: {e}");
        }
    };
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);
    
    let mut records = rdr.records();
    
    // Read the first record to determine the format
    if let Some(Ok(first_record)) = records.next() {
        if first_record.len() == 3 {
            // 3-column format - process this record and continue with the iterator
            let mut data = Vec::new();
            
            // Process the first record we already read
            if let (Ok(origin), Ok(destination), Ok(value)) = (
                first_record[0].parse::<i32>(),
                first_record[1].parse::<i32>(),
                first_record[2].parse::<f32>()
            ) {
                data.push((origin, destination, value));
            }
            
            // Process remaining records
            for record in records.filter_map(Result::ok) {
                if let (Ok(origin), Ok(destination), Ok(value)) = (
                    record[0].parse::<i32>(),
                    record[1].parse::<i32>(),
                    record[2].parse::<f32>()
                ) {
                    data.push((origin, destination, value));
                }
            }
            
            data
        } else {
            // Rectangular format - pass the first record and remaining iterator
            read_rectangular_csv_from_records(first_record, records)
        }
    } else {
        Vec::new()
    }
}

/// Reads a rectangular CSV from an already-started records iterator where the first row contains destinations
/// and the first column contains origins.
///
/// # Arguments
/// * `header_record` - The first record containing destinations
/// * `records` - Iterator over remaining CSV records
///
/// # Returns
/// A vector of tuples `(i32, i32, f32)` representing the origin, destination, and value.
fn read_rectangular_csv_from_records(
    header_record: csv::StringRecord,
    records: csv::StringRecordsIter<std::fs::File>
) -> Vec<(i32, i32, f32)> {
    // Parse the header row to get destinations
    let destinations: Vec<i32> = header_record.iter()
        .skip(1) // Skip the first column (it's empty or contains a label)
        .filter_map(|s| s.parse().ok())
        .collect();
    
    if destinations.is_empty() {
        return Vec::new();
    }
    
    let mut data = Vec::new();
    
    // Process each subsequent row
    for record in records.filter_map(Result::ok) {
        // Parse the origin from the first column
        if let Ok(origin) = record[0].parse::<i32>() {
            // Process each value in the row (skip first column)
            for (col_idx, value_str) in record.iter().skip(1).enumerate() {
                if col_idx < destinations.len() && 
                   let Ok(value) = value_str.parse::<f32>() && 
                   value != 0.0 {
                    data.push((origin, destinations[col_idx], value));
                }
            }
        }
    }
    
    data
}

/// Determines the complete list of zones either from the optional zones CSV file
/// or by extracting unique origins and destinations from the input data.
///
/// # Arguments
/// * `arg` - The command-line arguments.
/// * `data` - The vector of tuples `(i32, i32, f32)` representing the input data.
///
/// # Returns
/// A sorted vector of unique zone numbers.
fn get_all_zones(arg: &[String], data: &[(i32, i32, f32)]) -> Vec<i32> {
    if arg.len() > 3 {
        let zone_file = File::open(&arg[3]).unwrap();
        let mut zone_rdr = csv::Reader::from_reader(zone_file);
        let mut zones: Vec<i32> = zone_rdr
            .records()
            .filter_map(|result| result.ok()?.get(0)?.parse().ok())
            .collect();
        zones.sort_unstable();
        zones
    } else {
        let zones: HashSet<i32> = data
            .par_iter()
            .flat_map(|(origin, destination, _)| vec![*origin, *destination])
            .collect();
        let mut zones: Vec<i32> = zones.into_iter().collect();
        zones.sort_unstable();
        zones
    }
}

/// Builds a matrix of size `|origin| * |destination|` where each cell contains
/// the value corresponding to the origin and destination pair.
///
/// # Arguments
/// * `data` - The vector of tuples `(i32, i32, f32)` representing the input data.
/// * `all_zones` - The sorted vector of unique zone numbers.
///
/// # Returns
/// A vector of `f32` representing the flattened matrix.
fn build_matrix(data: &[(i32, i32, f32)], all_zones: &[i32]) -> Vec<f32> {
    let zone_count = all_zones.len();
    let zone_index: std::collections::HashMap<i32, usize> = all_zones
        .iter()
        .enumerate()
        .map(|(i, &zone)| (zone, i))
        .collect();

    let mut matrix = vec![0.0f32; zone_count * zone_count];
    for (origin, destination, value) in data {
        if let (Some(&origin_idx), Some(&destination_idx)) =
            (zone_index.get(origin), zone_index.get(destination))
        {
            matrix[origin_idx * zone_count + destination_idx] = *value;
        }
    }
    matrix
}

/// Writes the MTX file in the specified format. If the output file name ends with `.gz`,
/// the file is written as a gzip-compressed file.
///
/// # Arguments
/// * `output_file_name` - The path to the output MTX file.
/// * `all_zones` - The sorted vector of unique zone numbers.
/// * `matrix` - The flattened matrix of values.
///
/// # Panics
/// This function will panic if it fails to create or write to the output file.
fn write_mtx_file(output_file_name: &str, all_zones: &[i32], matrix: &[f32]) {
    let output_file = File::create(output_file_name).unwrap();
    let mut writer: Box<dyn Write> = if output_file_name.ends_with(".gz") {
        Box::new(BufWriter::new(GzEncoder::new(output_file, Compression::default())))
    } else {
        Box::new(BufWriter::new(output_file))
    };

    let zone_count = all_zones.len() as i32;

    writer.write_all(&0xC4D4F1B2u32.to_le_bytes()).unwrap(); // Magic Number
    writer.write_all(&1i32.to_le_bytes()).unwrap(); // Version Number
    writer.write_all(&1i32.to_le_bytes()).unwrap(); // Type
    writer.write_all(&2i32.to_le_bytes()).unwrap(); // Dimensions
    writer.write_all(&zone_count.to_le_bytes()).unwrap(); // Index size for origin
    writer.write_all(&zone_count.to_le_bytes()).unwrap(); // Index size for destination

    let is_little_endian = cfg!(target_endian = "little");

    if is_little_endian {
        // Write all origin zone numbers in a single call 
        let origin_zone_bytes: &[u8] = bytemuck::cast_slice(all_zones);
        writer.write_all(origin_zone_bytes).unwrap(); // Zone Numbers for Origin

        // Write all destination zone numbers in a single call
        writer.write_all(origin_zone_bytes).unwrap(); // Zone Numbers for Destination

        // Write all matrix values in a single call
        let matrix_bytes: &[u8] = bytemuck::cast_slice(matrix);
        writer.write_all(matrix_bytes).unwrap();

    } else {
        // Convert all_zones to little-endian
        let origin_zone_bytes: Vec<u8> = all_zones
            .par_iter()
            .flat_map(|&zone| zone.to_le_bytes())
            .collect();
        writer.write_all(&origin_zone_bytes).unwrap(); // Zone Numbers for Origin
        writer.write_all(&origin_zone_bytes).unwrap(); // Zone Numbers for Destination

        // Convert matrix to little-endian
        let matrix_bytes: Vec<u8> = matrix
            .par_iter()
            .flat_map(|&value| value.to_le_bytes())
            .collect();
        writer.write_all(&matrix_bytes).unwrap();
    }
}
