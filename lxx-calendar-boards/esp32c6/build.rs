use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    linker_be_nice();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    println!("cargo:rustc-link-arg=-Tlinkall.x");

    handle_partition_table();
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                what if what.starts_with("_defmt_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                what if what.starts_with("esp_rtos_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                "free"
                | "malloc"
                | "calloc"
                | "get_free_internal_heap_size"
                | "malloc_internal"
                | "realloc_internal"
                | "calloc_internal"
                | "free_internal" => {
                    eprintln!();
                    eprintln!(
                        "💡 Did you forget the `esp-alloc` dependency or didn't enable the `compat` feature on it?"
                    );
                    eprintln!();
                }
                _ => (),
            },
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}

fn handle_partition_table() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let partition_csv = manifest_dir.join("partitions.csv");
    let partition_bin = out_dir.join("partitions.bin");

    if partition_csv.exists() {
        println!("cargo:rerun-if-changed=partitions.csv");

        let csv_content =
            fs::read_to_string(&partition_csv).expect("Failed to read partitions.csv");

        let mut partitions: Vec<PartitionEntry> = Vec::new();

        for line in csv_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("Name,") {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 5 {
                let entry = PartitionEntry {
                    name: parts[0].trim().to_string(),
                    partition_type: parse_partition_type(parts[1].trim()),
                    sub_type: parse_partition_subtype(parts[2].trim()),
                    offset: parse_hex_or_int(parts[3].trim()),
                    size: parse_hex_or_int(parts[4].trim()),
                };
                partitions.push(entry);
            }
        }

        let mut bin_data = Vec::new();

        bin_data.extend_from_slice(b"ESP32");
        bin_data.push(0xFF);
        bin_data.push(0x00);
        bin_data.push(partitions.len() as u8);

        for partition in &partitions {
            bin_data.extend_from_slice(&partition.encode());
        }

        let checksum = calculate_checksum(&bin_data);
        bin_data.push(checksum);

        fs::write(&partition_bin, &bin_data).expect("Failed to write partitions.bin");

        println!(
            "cargo:rustc-env=PARTITION_TABLE_PATH={}",
            partition_bin.display()
        );

        println!("cargo:warning=Partition table generated:");
        for p in &partitions {
            println!(
                "cargo:warning=  {} @ 0x{:X}, size: 0x{:X}",
                p.name, p.offset, p.size
            );
        }
    }
}

struct PartitionEntry {
    name: String,
    partition_type: u8,
    sub_type: u8,
    offset: u32,
    size: u32,
}

impl PartitionEntry {
    fn encode(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];

        buf[0] = self.partition_type;
        buf[1] = self.sub_type;
        buf[2..6].copy_from_slice(&self.offset.to_le_bytes());
        buf[6..10].copy_from_slice(&self.size.to_le_bytes());

        let name_bytes = self.name.as_bytes();
        let len = name_bytes.len().min(16);
        buf[10..10 + len].copy_from_slice(&name_bytes[..len]);

        buf[26..30].copy_from_slice(&[0xFF; 4]);
        buf[30..32].copy_from_slice(&[0x00, 0x00]);

        buf
    }
}

fn parse_hex_or_int(s: &str) -> u32 {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16).unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

fn parse_partition_type(s: &str) -> u8 {
    match s.trim() {
        "data" => 0x01,
        "app" => 0x00,
        _ => parse_hex_or_int(s) as u8,
    }
}

fn parse_partition_subtype(s: &str) -> u8 {
    match s.trim() {
        "nvs" => 0x01,
        "phy" => 0x01,
        "factory" => 0x00,
        "ota_0" => 0x10,
        "ota_1" => 0x11,
        "ota" => 0x00,
        "spiffs" => 0x82,
        _ => parse_hex_or_int(s) as u8,
    }
}

fn calculate_checksum(data: &[u8]) -> u8 {
    !data.iter().fold(0xEFu8, |acc, &b| acc ^ b)
}
