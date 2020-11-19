use num_format::{Locale, ToFormattedString};
use std::io::{BufReader, prelude::*};

struct Compression {
	compressor: String,
	bytes: Option<u64>,
	time_pack: Option<String>,
	time_unpack: Option<String>,
}

struct Benchmark {
	name: String,
	compressions: Vec<Compression>,
}

fn get_compressor<'c>(compressions: &'c mut Vec<Compression>, name: &str) -> &'c mut Compression {
	let pos = if let Some(pos) = compressions.iter_mut().rposition(|c: &mut Compression| (*c).compressor == name) {
		pos
	} else {
		compressions.push(Compression { 
			compressor: name.to_string(),
			bytes: None,
			time_pack: None,
			time_unpack: None,
		});
		compressions.len() - 1
	};
	compressions.get_mut(pos).unwrap()
}

fn main() -> std::io::Result<()> {
	let mut benchmarks: Vec<Benchmark> = Vec::new();
    for file in std::fs::read_dir(".")? {
		let file = file?;
		let path = file.path();
		if path.extension().map_or(false, |s| s == "bench") {
			let bench_name = path.file_stem().unwrap().to_str().unwrap().strip_prefix("rust-compression-").unwrap();
			let mut compressions = Vec::new();
			let mut comp_name = None;
			let mut unpack = false;
			let f = std::fs::File::open(&path)?;
			let r = BufReader::new(f);
			for line in r.lines() {
				let line = line?;
				if let Some(prefix) = line.strip_suffix(" bytes") {
					let mut parts = prefix.split(": ");
					if let (Some(compressor), Some(bytes)) = (parts.next(), parts.next()) {
						get_compressor(&mut compressions, compressor).bytes = Some(str::parse(bytes).unwrap());
					}
				} else {
					if let Some(suffix) = line.strip_prefix("compression/") {
						let identifier = if let Some(pos) = suffix.find(' ') {
							&suffix[..pos]
						} else {
							suffix
						};
						let mut parts = identifier.split(".");
						if let (Some(compressor), Some(pack)) = (parts.next(), parts.next()) {
							comp_name = if let Some(crc) = parts.next() {
								if crc == "crc" {
									Some(format!("{} + crc", compressor))
								} else {
									Some(compressor.to_string())
								}
							} else {
								Some(compressor.to_string())
							};
							unpack = pack == "unpack";
						}
					}
					static TIME_PREFIX: &str = "time:   [";
					if let Some(pos) = line.find(TIME_PREFIX) {
						let suffix = &line[(pos + TIME_PREFIX.len())..];
						let comp = get_compressor(&mut compressions, comp_name.as_ref().unwrap());
						let mut parts = suffix.split(" ");
						let time = parts.nth(2).unwrap();
						let unit = parts.next().unwrap();
						let time_unit = format!("{} {}", time, unit);
						if unpack {
							let prev = std::mem::replace(&mut comp.time_unpack, Some(time_unit));
							assert_eq!(None, prev, "{}: double comp[{}.time_unpack", bench_name, &comp.compressor);
						} else {
							let prev = std::mem::replace(&mut comp.time_pack, Some(time_unit));
							assert_eq!(None, prev, "{}: double comp[{}.time_pack", bench_name, &comp.compressor);
						}
					}
				}
			}
			benchmarks.push(Benchmark {
				name: bench_name.to_string(),
				compressions,
			});
		}
	}

	println!("|benchmarks|{}|",
		benchmarks.iter().map(|b| format!("{} ↘|bytes|↗", b.name)).collect::<Vec<_>>().join("|"));
	for (i, comp) in benchmarks[0].compressions.iter().enumerate() {
		print!("|{}", comp.compressor);
		for benchmark in &benchmarks {
			let comp = &benchmark.compressions[i];
			print!(
				"|{}|{} b|{}",
				comp.time_pack.as_ref().map_or("—", |s| s),
				comp.bytes.unwrap_or(0).to_formatted_string(&Locale::eu),
				comp.time_unpack.as_ref().map_or("—", |s| s),
			);
		}
		println!("|");
	}
	Ok(())
}
