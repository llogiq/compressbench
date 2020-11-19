use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::io::{Cursor, Read, Write};

fn compare_compress(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
	// read into memory first
	let uncompressed = std::fs::read(
		std::env::var("FILE_TO_COMPRESS").expect("set $FILE_TO_COMPRESS")
	).expect("reading $FILE_TO_COMPRESS");
	let orig_len = uncompressed.len();
	
	println!("uncompressed: {} bytes", orig_len);
	let mut compressed = Vec::with_capacity(orig_len);
	let mut unpacked = Vec::with_capacity(orig_len);
	// various LZ4 implementtions
    group.bench_function("lz4-compression.pack", |b| {
		let orig = &uncompressed;
        b.iter(|| {
            black_box(&mut compressed).clear();
            lz4_compression::compress::compress_into(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("lz4-compression: {} bytes", compressed.len());
    group.bench_function("lz4-compression.unpack", |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lz4_compression::decompress::decompress_into(black_box(&compressed), black_box(&mut unpacked))
        })
    });
    group.bench_function("lz4_flex.pack", |b| {
		let orig = &uncompressed;
        b.iter(|| {
            black_box(&mut compressed).clear();
            lz4_flex::compress_into(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("lz4_flex: {} bytes", compressed.len());
    group.bench_function("lz4_flex.unpack", |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lz4_flex::decompress_into(black_box(&compressed), black_box(&mut unpacked))
        })
    });
    group.bench_function("lz_fear.pack", |b| {
		let lzfear = lz_fear::CompressionSettings::default();
		let orig = &uncompressed[..];
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzfear.compress(black_box(orig), black_box(&mut compressed))
        })
    });
    println!("lz_fear: {} bytes", compressed.len());
    group.bench_function("lz_fear.unpack", |b| {
        b.iter(|| {
            let mut lzfear = lz_fear::LZ4FrameReader::new(black_box(&compressed[..])).unwrap().into_read();
            black_box(&mut unpacked).clear();
            lzfear.read_to_end(black_box(&mut unpacked))
        })
    });
    {
		use lzzzz::lz4::{compress_to_vec, decompress, ACC_LEVEL_DEFAULT};
		group.bench_function("lzzzz.pack", |b| {
			let orig = &uncompressed[..];
			b.iter(|| {
				black_box(&mut compressed).clear();
				compress_to_vec(black_box(orig), black_box(&mut compressed), ACC_LEVEL_DEFAULT)
			})
		});
		println!("lzzzz: {} bytes", compressed.len());
		group.bench_function("lzzzz.unpack", |b| {
			b.iter(|| {
				black_box(&mut unpacked).clear();
				decompress(black_box(&compressed), black_box(&mut unpacked))
			})
		});
	}
	// zstandard
	for level in 1..10 {
		let orig = &uncompressed;
		compressed.clear();
		compressed.resize(orig_len * 2, 0_u8);
		let comp_len = zstd::block::compress_to_buffer(orig, &mut compressed, level).unwrap();
		group.bench_function(&format!("zstd-level-{}.pack", level), |b| {
			b.iter(|| {
				zstd::block::compress_to_buffer(black_box(orig), black_box(&mut compressed), black_box(level))
			})
		});
		println!("zstd-level-{}: {} bytes", level, comp_len);
		unpacked.clear();
		unpacked.resize(orig_len, 0_u8);
		let comp = &compressed[..comp_len];
		group.bench_function(&format!("zstd-level-{}.unpack", level), |b| {
			b.iter(|| {
				black_box(&mut unpacked).clear();
				zstd::block::decompress_to_buffer(black_box(comp), black_box(&mut unpacked))
			})
		});
	}
	// snappy
	let orig = &uncompressed;
	group.bench_function("snap.pack", |b| {
		compressed.clear();
		compressed.resize(orig_len * 2, 0_u8);
        b.iter(|| {
			snap::raw::Encoder::new().compress(black_box(orig), black_box(&mut compressed))
        })
    });
    let comp_len = snap::raw::Encoder::new().compress(orig, &mut compressed).unwrap();
    println!("snap: {} bytes", comp_len);
    group.bench_function("snap.unpack", |b| {
		unpacked.clear();
		unpacked.resize(orig_len, 0_u8);
		let comp = &compressed[..comp_len];
        b.iter(|| {
            snap::raw::Decoder::new().decompress(black_box(comp), black_box(&mut unpacked))
        })
    });
    group.bench_function("snappy-framed.pack", |b| {
        b.iter(|| {
			black_box(&mut compressed).clear();
			let mut enc = snappy_framed::write::SnappyFramedEncoder::new(black_box(&mut compressed)).unwrap();
			enc.write_all(black_box(orig)).unwrap();
			enc.flush().unwrap();
        })
    });
    println!("snappy-framed: {} bytes", compressed.len());
    group.bench_function("snappy-framed.unpack.nocrc", |b| {
        b.iter(|| {
			black_box(&mut unpacked).clear();
			let mut cursor = Cursor::new(black_box(&compressed));
			let mut decoder = snappy_framed::read::SnappyFramedDecoder::new(&mut cursor, snappy_framed::read::CrcMode::Ignore);
			decoder.read_to_end(black_box(&mut unpacked))
        })
    });
    group.bench_function("snappy-framed.unpack.crc", |b| {
        b.iter(|| {
			black_box(&mut unpacked).clear();
			let mut cursor = Cursor::new(&compressed);
			let mut decoder = snappy_framed::read::SnappyFramedDecoder::new(&mut cursor, snappy_framed::read::CrcMode::Verify);
            decoder.read_to_end(black_box(&mut unpacked))
        })
    });
    // deflate
    use deflate::Compression;
    for &level in &[Compression::Fast, Compression::Default, Compression::Best] {
		group.bench_function(&format!("deflate-{:?}.pack", level), |b| {
			let orig = &uncompressed;
			b.iter(|| {
				black_box(&mut compressed).clear();
				let mut encoder = deflate::write::DeflateEncoder::new(black_box(std::mem::replace(&mut compressed, Vec::new())), level);
				encoder.write_all(black_box(orig)).unwrap();
				compressed = black_box(encoder.finish().unwrap());
			})
		});
		println!("deflate-{:?}: {} bytes", level, compressed.len());
		group.bench_function(&format!("deflate-{:?}.unpack", level), |b| {
			b.iter(|| {
				deflate::deflate_bytes(black_box(&compressed))
			})
		});
	}
	for level in flate2::Compression::fast().level() .. flate2::Compression::best().level() {
		group.bench_function(&format!("flate2-{}.pack", level), |b| {
			compressed.clear();
			compressed.resize(orig_len * 2, 0_u8);
			b.iter(|| {
				flate2::Compress::new(flate2::Compression::new(level), false).compress(
					black_box(orig), 
					black_box(&mut compressed),
					flate2::FlushCompress::Finish
				)
			})
		});
		let mut comp = flate2::Compress::new(flate2::Compression::new(level), false);
		comp.compress(
			black_box(orig), 
			black_box(&mut compressed),
			flate2::FlushCompress::Finish
		).unwrap();
		println!("flate2-{}: {} bytes", level, comp.total_out());
		group.bench_function(&format!("flate2-{}.unpack", level), |b| {
			unpacked.clear();
			unpacked.resize(orig_len, 0_u8);
			let comp = &compressed[..comp_len];
			b.iter(|| {
				flate2::Decompress::new(false).decompress(
					black_box(comp),
					black_box(&mut unpacked),
					flate2::FlushDecompress::Finish
				)
			})
		});
	}
	use yazi::CompressionLevel;
	for &level in &[CompressionLevel::BestSpeed, CompressionLevel::Default, CompressionLevel::BestSize] {
		group.bench_function(&format!("yazi-{:?}.pack", level), |b| {
			let orig = &uncompressed[..];
			b.iter(|| {
				black_box(&mut compressed).clear();
				let mut encoder = yazi::Encoder::new();
				encoder.set_format(yazi::Format::Raw);
				encoder.set_level(level);
				let mut stream = encoder.stream_into_vec(black_box(&mut compressed));
				stream.write(black_box(orig)).unwrap();
				stream.finish()
			})
		});
		println!("yazi-{:?}: {} bytes", level, compressed.len());
		group.bench_function(&format!("yazi-{:?}.unpack", level), |b| {
			b.iter(|| {
				black_box(&mut unpacked).clear();
				let mut decoder = yazi::Decoder::new();
				decoder.set_format(yazi::Format::Raw);
				let mut stream = decoder.stream_into_vec(&mut unpacked);
				stream.write(black_box(&compressed)).unwrap();
				stream.finish()
			})
		});
	}
	// LZMA
	group.bench_function("lzma-rs.pack", |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::lzma_compress(black_box(&mut &uncompressed[..]), black_box(&mut compressed))
        })
    });
    println!("lzma-rs: {} bytes", compressed.len());
    group.bench_function("lzma-rs.unpack", |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lzma_rs::lzma_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked))
        })
    });
    group.bench_function("lzma-rs/2.pack", |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::lzma2_compress(black_box(&mut &uncompressed[..]), black_box(&mut compressed))
        })
    });
    println!("lzma-rs/2: {} bytes", compressed.len());
    group.bench_function("lzma-rs/2.unpack", |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lzma_rs::lzma2_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked))
        })
    });
    group.bench_function("lzma-rs/xz.pack", |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            lzma_rs::xz_compress(black_box(&mut &uncompressed[..]), black_box(&mut compressed))
        })
    });
    println!("lzma-rs/xz: {} bytes", compressed.len());
    group.bench_function("lzma-rs/xz.unpack", |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            lzma_rs::xz_decompress(black_box(&mut &compressed[..]), black_box(&mut unpacked))
        })
    });
    // Zopfli
	group.bench_function("zopfli.pack", |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            zopfli::compress(
				&zopfli::Options::default(),
				&zopfli::Format::Deflate,
				black_box(&mut &uncompressed[..]),
				black_box(&mut compressed)
			).unwrap()
        })
    });
    println!("zopfli: {} bytes", compressed.len());
    // no decompression here
    // Brotli
    group.bench_function("brotli.pack", |b| {
        b.iter(|| {
            black_box(&mut compressed).clear();
            brotli::BrotliCompress(
				black_box(&mut &uncompressed[..]),
				black_box(&mut compressed),
				&Default::default(),
			)
        })
    });
    println!("brotli: {} bytes", compressed.len());
    group.bench_function("brotli.unpack", |b| {
        b.iter(|| {
            black_box(&mut unpacked).clear();
            brotli::BrotliDecompress(black_box(&mut &compressed[..]), black_box(&mut unpacked))
        })
    });
    // tar
    group.bench_function("tar.pack", |b| {
		compressed.reserve(uncompressed.len()); // double the excitement! ;P
        b.iter(|| {
			black_box(&mut compressed).clear();
			use tar::{Builder, Header};

			let mut header = Header::new_gnu();
			header.set_path("data").unwrap();
			header.set_size(4);
			header.set_cksum();

			let mut ar = Builder::new(std::mem::replace(&mut compressed, Vec::new()));
			ar.append(&header, &orig[..]).unwrap();
			compressed = ar.into_inner().unwrap();
			compressed.len()
		})
	});
	println!("tar: {} bytes", compressed.len());
	// unfortunately, tar only unpacks to disk, so this result would be
	// incomparable. Perhaps in a later benchmark.
    // zip
	group.bench_function("zip.pack", |b| {
		compressed.clear();
		compressed.resize(orig_len * 2, 0_u8);
        b.iter(|| {
			let mut zipw = zip::ZipWriter::new(Cursor::new(black_box(&mut compressed)));
			zipw.start_file("data", zip::write::FileOptions::default()).unwrap();
			zipw.write(black_box(orig)).unwrap();
			zipw.finish().unwrap();
		})
	});
	let ziplen = {
		let mut zipw = zip::ZipWriter::new(Cursor::new(&mut compressed));
		zipw.start_file("data", zip::write::FileOptions::default()).unwrap();
		zipw.write(orig).unwrap();
		let c = zipw.finish().unwrap();
		c.position() as usize
	};
	println!("zip: {} bytes", ziplen);
	group.bench_function("zip.unpack", |b| {
		b.iter(|| {
			black_box(&mut unpacked).clear();
			let mut zipr = zip::ZipArchive::new(Cursor::new(black_box(&compressed[..ziplen]))).unwrap();
			zipr.by_index(0).unwrap().read_to_end(black_box(&mut unpacked)).unwrap();
		})
	});
    group.finish();
}

criterion_group!(
	name = benches;
	config = Criterion::default().sample_size(10);
	targets = compare_compress);
criterion_main!(benches);
