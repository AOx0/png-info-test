mod byte_iterator;
use byte_iterator::ContentIter;

mod args;
use args::Args;

use std::io::Read;

fn deflate(vec: &[u8]) -> Result<Vec<u8>, String> {
    let result = vec.to_vec();

    Ok(result)
}

fn app() -> Result<(), String> {
    let Args { file } = Args::get()?;

    let png_bytes = read_file(&file)?;
    let mut iterator = ContentIter::new(&png_bytes);

    static MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let magic_bytes = iterator.next_slice_of(8, "PNG Magic number")?;
    if magic_bytes != MAGIC {
        Err(format!(
            "File {} is not a PNG image.\nMagic number does not match:\n    Expected: {}\n       Found: {}",
            file.display(),
            MAGIC[..].iter().map(|val| format!("{val:0>2x}").to_uppercase()).collect::<Vec<_>>().join(" "),
            magic_bytes.iter().map(|val| format!("{val:0>2x}").to_uppercase()).collect::<Vec<_>>().join(" ")
        ))?;
    }

    static IHDR_BLOCK: [u8; 8] = [0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52];
    let ihdr_bytes = iterator.next_slice_of(8, "IHDR Chunk")?;
    if ihdr_bytes != IHDR_BLOCK {
        Err(format!(
            "Expected 13 byte long IHDR block signature in file {}.\n    Expected: {}\n       Found: {}",
            file.display(),
            MAGIC[..]
                .iter()
                .map(|val| format!("{val:0>2x}").to_uppercase())
                .collect::<Vec<_>>()
                .join(" "),
            ihdr_bytes
                .iter()
                .map(|val| format!("{val:0>2x}").to_uppercase())
                .collect::<Vec<_>>()
                .join(" ")
        ))?;
    }

    let width = iterator.read_u32_be("PNG Width")?;
    let height = iterator.read_u32_be("PNG Height")?;
    let bit_depth = iterator.next_slice_of(1, "PNG Bit Depth")?[0];
    let color_type = iterator.next_slice_of(1, "PNG Color Type")?[0];

    let (channels, bits_per_channel) = match (color_type, bit_depth) {
        (0, 1 | 2 | 4 | 8 | 16) => (1, bit_depth),
        (2, 8 | 16) => (3, bit_depth * 3),
        (3, 1 | 2 | 4 | 8) => (1, bit_depth),
        (4, 8 | 16) => (2, bit_depth * 2),
        (6, 8 | 16) => (4, bit_depth * 4),
        _ => {
            return Err(format!("Combination of Color Type {color_type} and Bit Depth {bit_depth} is not supported for PNG.\
                 See https://en.wikipedia.org/wiki/Portable_Network_Graphics"));
        }
    };

    let compression_method = iterator.next_slice_of(1, "PNG Compression Method")?[0];
    let filter_method = iterator.next_slice_of(1, "PNG Filter Method")?[0];
    let interlace_method = iterator.next_slice_of(1, "PNG Interlace Method")?[0];

    println!("             Width: {width:?}");
    println!("            Height: {height:?}");
    println!("");
    println!("         Bit Depth: {bit_depth:?}");
    println!("        Color Type: {color_type:?}");
    println!("          Channels: {channels}");
    println!("  Bits per channel: {bits_per_channel}");
    println!("");
    println!("Compression Method: {compression_method:?}");
    println!("     Filter Method: {filter_method:?}");
    println!("  Interlace Method: {interlace_method:?}");
    println!("");

    // Skip IHDR CRC
    iterator.next_slice_of(4, "IHDR CRC")?;

    while let Ok(size) = iterator.read_u32_be("Chunk Size") {
        let chunk_type = iterator.read_utf8_str(4, "Chunk Type")?;
        let critical = chunk_type
            .chars()
            .next()
            .expect("We created it, we expect 4 len always")
            .is_uppercase();
        println!(
            "\nChunk {chunk_type} ({}) of size {size} at address 0x{:0>2x}",
            if critical { "Critical" } else { "Ancillary" },
            iterator.get_address()
        );
        // Skip chunk size
        let data_bytes = iterator.next_slice_of(size as usize, "Chunk Data")?;

        if critical && chunk_type == "IDAT" {
            let mut data_iterator = ContentIter::new(&data_bytes);
            let is_last = data_iterator.next_bit("IS_LAST")?;
            let block_type = data_iterator.next_bit_slice_of(2, "BTYPE")?;

            println!("           IS_LAST: {:b}", is_last);
            println!(
                "             BTYPE: {}",
                block_type
                    .iter()
                    .map(|v| format!("{v:b}"))
                    .collect::<Vec<_>>()
                    .join("")
            );

            if block_type == &[0, 0] {
                let padding = data_iterator.skip_remaining_bits()?;

                if padding.iter().sum::<u8>() != 0 {
                    return Err(format!(
                        "Non zero padding {}",
                        padding
                            .into_iter()
                            .map(|v| format!("{v}"))
                            .collect::<Vec<_>>()
                            .join("")
                    ));
                }

                let len = data_iterator.read_u16_be("DEFLATE Block Lenght")?;

                println!("Len: {len:b}");
            }
        }

        // Skip chunk CRC
        iterator.next_slice_of(4, "Chunk CRC")?;
    }

    Ok(())
}

fn read_file(file: &std::path::Path) -> Result<Vec<u8>, String> {
    let mut file_handler = std::fs::OpenOptions::new()
        .read(true)
        .write(false)
        .append(false)
        .truncate(false)
        .open(&file)
        .map_err(|err| format!("{err}"))?;

    let file_size = file_handler
        .metadata()
        .map_err(|err| format!("{err}"))?
        .len() as usize;
    let mut contents = Vec::with_capacity(file_size);
    file_handler
        .read_to_end(&mut contents)
        .map_err(|err| format!("{err}"))?;

    Ok(contents)
}

fn main() {
    if let Err(err) = app() {
        println!("Error: {err}");
    }
}
