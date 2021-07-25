use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::str;
use std::time::Instant;

use byteorder::{ByteOrder, LittleEndian};
use clap::{crate_version, App, AppSettings, Arg, ArgMatches};

static EOI: [u8; 2] = [255, 217];

fn main() {
    let start = Instant::now();

    let matches = parse_args();

    let silent = matches.is_present("silent");
    let in_place = matches.is_present("in-place");
    let dry_run = matches.is_present("dry-run");
    let force = matches.is_present("force");

    let inputs: Vec<&str> = matches.values_of("INPUT").unwrap().collect();
    let output_opt = matches.value_of("OUTPUT");

    let (total, size) = inputs
        .iter()
        .map(|input| {
            if in_place {
                convert_file(input, input, silent, dry_run, force)
            } else if dry_run {
                convert_file(input, ".test.jpg", silent, dry_run, force)
            } else {
                let output = output_opt.unwrap();
                convert_file(input, output, silent, dry_run, force)
            }
        })
        .fold((0, 0), |(acc_t, acc_s), (t, s)| (acc_t + t, acc_s + s));

    if !silent {
        println!(
            "---\nExtracted {} of {} MB ({} %) -- Time: {:.3} s",
            (size as f32 / 1_000_000.).round(),
            (total as f32 / 1_000_000.).round(),
            (100. * (size as f32 / total as f32)).round(),
            start.elapsed().as_millis() as f32 / 1_000.,
        )
    }
}

fn parse_args() -> ArgMatches<'static> {
    App::new("sampan")
        .version(crate_version!())
        .author("Francesco Pasa <francescopasa@gmail.com>")
        .about("Strips unnecessary information from Samsung panorama images.")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name("INPUT")
                .help(
                    "JPEG file to strip. Can be a pattern containing \
                    wildcards to process multiple files at once. \
                    Use ** to recurse into folders. Wildcard expansion \
                    only works if the shell support it.",
                )
                .required(true)
                .takes_value(true)
                .multiple(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .help("Output file name.")
                .required(true)
                .takes_value(true)
                .conflicts_with_all(&["in-place", "dry-run"]),
        )
        .arg(
            Arg::with_name("silent")
                .short("s")
                .long("silent")
                .takes_value(false)
                .help("Do not print any information."),
        )
        .arg(
            Arg::with_name("in-place")
                .short("i")
                .long("in-place")
                .takes_value(false)
                .help(
                    "Overwrites file(s) in place. Ensure you have a backup. \
                    If this is enabled, the OUTPUT argument must not be set.",
                ),
        )
        .arg(
            Arg::with_name("dry-run")
                .short("d")
                .long("dry-run")
                .takes_value(false)
                .help("Run program but do not write output."),
        )
        .arg(
            Arg::with_name("force")
                .short("f")
                .long("force")
                .takes_value(false)
                .help(
                    "Force conversion even if the Samsung version number \
                    is unsupported and even if the entity types do match. \
                    This option can be helpful for forcing conversion of \
                    some files, and will never touch files that are not in \
                    Samsung format.",
                ),
        )
        .get_matches()
}

fn convert_file(
    input: &str,
    output: &str,
    silent: bool,
    dry_run: bool,
    force: bool,
) -> (usize, usize) {
    let mut file = File::open(input).expect(&format!("Cannot open file {}", input));
    let total = file.metadata().unwrap().len() as usize;

    let entries_count = match read_entries_count(&mut file, force) {
        Ok(count) => count,
        Err(err) => {
            println!("{}: {}", input, err);
            return (total, total);
        }
    };

    // Find smallest offset of the custom data that is appended to the JPEG,
    // which we will then proceed to eliminate.
    let mut smallest_offset = total;
    for i in 0..entries_count {
        match read_entry_offset(&mut file, i, entries_count, force) {
            Ok(offset) => {
                if offset < smallest_offset {
                    smallest_offset = offset
                }
            }
            Err(err) => {
                println!("{}: {}", input, err);
                return (total, total);
            }
        }
    }

    if !silent {
        println!(
            "{} -> {}\n  Extracting {:.1} of {:.1} MB ({} %)",
            input,
            output,
            smallest_offset as f32 / 1_000_000.,
            total as f32 / 1_000_000.,
            (100. * (smallest_offset as f32 / total as f32)).round(),
        );
    }

    if !dry_run {
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut jpeg_data = vec![0; smallest_offset];
        file.read(&mut jpeg_data).unwrap();

        let mut output_file = File::create(output).expect(&format!("Cannot open file {}", output));
        output_file
            .write_all(&jpeg_data)
            .expect(&format!("Cannot write file {}", output));

        if &jpeg_data[jpeg_data.len() - 2..] != &EOI {
            output_file
                .write(&EOI)
                .expect(&format!("Cannot write file {}", output));
        }
    }

    (total, smallest_offset)
}

fn read_entries_count(file: &mut File, force: bool) -> Result<i64, String> {
    let mut footer = [0; 8];

    file.seek(SeekFrom::End(-8)).unwrap();
    file.read(&mut footer).unwrap();

    // ascii for SEFT
    if footer[4..] != [83, 69, 70, 84] {
        return Err("Image is not a Samsung panorama. [skip]".to_string());
    }

    let length = LittleEndian::read_u32(&footer[..4]) as i64;
    file.seek(SeekFrom::End(-8 - length)).unwrap();

    let mut entries_header = [0; 12];
    file.read(&mut entries_header).unwrap();

    // ascii for SEFH
    if entries_header[0..4] != [83, 69, 70, 72] {
        return Err("Image is not a Samsung panorama. [skip]".to_string());
    }

    let version = LittleEndian::read_u32(&entries_header[4..8]) as i64;
    if !force && version != 101 && version != 103 && version != 105 && version != 106 {
        return Err(format!(
            "Unknown panorama version {}, \
            sampan only supports version 101, 103, 105, 106 [skip]",
            version
        ));
    }

    Ok(LittleEndian::read_u32(&entries_header[8..12]) as i64)
}

fn read_entry_offset(file: &mut File, n: i64, count: i64, force: bool) -> Result<usize, String> {
    let footer_len = 20 + count * 12;
    let mut header = [0; 8];

    file.seek(SeekFrom::End(12 - footer_len + n * 12)).unwrap();
    file.read(&mut header).unwrap();

    let type_ = LittleEndian::read_u16(&header[2..4]);
    let offset = LittleEndian::read_u32(&header[4..8]) as i64;

    let offset_from_end = footer_len + offset;
    file.seek(SeekFrom::End(-offset_from_end)).unwrap();
    let mut data = [0; 4];
    file.read(&mut data).unwrap();

    let data_type = LittleEndian::read_u16(&data[2..4]);
    if !force && data_type != type_ {
        return Err(format!(
            "Image is corrupted, entry types do not match: {} != {}. [skip]",
            data_type, type_
        ));
    }

    Ok(file.metadata().unwrap().len() as usize - offset_from_end as usize)
}
