use std::{
    fs::{self, File},
    io::{self, Error, Read, Seek},
    path::Path,
};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

const NON_XML_FILES: &[&str] = &[
    "manifest_xml",
    "COPYING",
    "Errors",
    "ExplainUnzip",
    "cloth_bazaar",
    "Cursors",
    "Dialogs",
    "Keyboard",
    "LICENSE",
    "LineBreaking Following Characters",
    "LineBreaking Leading Characters",
    "manifest_json",
    "spritesheetf",
    "iso_4217",
    "data",
    "manifest",
    "BillingMode",
];

pub fn extract_assets(path: &Path) -> io::Result<()> {
    let mut file = File::open(path)?;

    let real_size = file.metadata().unwrap().len();

    // this is all written for version 22+ by the way
    // if you have older version then idk..

    file.read_exact(&mut [0; 4 * 2])?; // 2 ints
    let version = file.read_i32::<BigEndian>()?;
    file.read_exact(&mut [0; 4 * 1])?; // int
    let big_endian = file.read_u8()? != 0;
    file.read_exact(&mut [0; 3])?;
    let metadata_size = file.read_u32::<BigEndian>()? as u64;
    let file_size = file.read_u64::<BigEndian>()?;
    let data_offset = file.read_u64::<BigEndian>()?;
    file.read_i64::<BigEndian>()?; // unknown

    // Some wack ass sanity tests, I didn't write these - stolen
    if version > 100
        || file_size > real_size
        || metadata_size > real_size
        || (version as u64) > real_size
        || data_offset > real_size
        || file_size < metadata_size
        || file_size < data_offset
    {
        return Err(Error::new(
            io::ErrorKind::InvalidData,
            "invalid assets file",
        ));
    }

    // bro i am not gonna waste my time trying to support both if only little endian is ever going to be used
    // If you ever get this error, you can add support for big endian by reading all data from this point
    // in big endian
    if big_endian {
        return Err(Error::new(
            io::ErrorKind::Unsupported,
            "big endian not supported.",
        ));
    }

    // NUL-terminated string LOL 😂
    read_nul_terminated_string(&mut file)?; // unity version

    file.read_u32::<LittleEndian>()?; // target_platform
    let enable_type_tree = file.read_u8()? != 0;

    if enable_type_tree {
        return Err(Error::new(
            io::ErrorKind::Unsupported,
            "enable_type_tree not supported.",
        ));
    }

    // Types
    let types_count = file.read_u32::<LittleEndian>()? as usize;
    let mut types = vec![0; types_count];
    for i in 0..types_count {
        let class_id = file.read_i32::<LittleEndian>()?;
        file.read_exact(&mut [0; 1 + 2])?; // is_stripped_type + script_type_index
        if class_id == 114 {
            file.read_exact(&mut [0; 16])?; // script_id
        }
        file.read_exact(&mut [0; 16])?; // old_type_hash

        types[i] = class_id;
    }

    // Objects
    let object_count = file.read_u32::<LittleEndian>()?;
    println!("Reading {object_count} objects from assets file.");
    for i in 0..object_count {
        if i != 0 && i % (object_count / 5) == 0 {
            println!("{i} / {object_count} objects read...");
        }

        // align the stream
        align_stream(&mut file)?;

        file.read_i64::<LittleEndian>()?; // path_id

        let byte_start = file.read_u64::<LittleEndian>()? + data_offset;

        // let byte_size_offset = position + bytes_to_skip + 8 * 2;
        file.read_u32::<LittleEndian>()?; // byte_size

        let type_id = file.read_u32::<LittleEndian>()?;
        let class_id = types[type_id as usize];

        if class_id != 49 {
            // 49 is TextAsset - the only one we need
            continue;
        }

        // now we gotta jump to the actual object data to read it, and then jump back for next iteration
        let position = file.seek(io::SeekFrom::Current(0))?;

        file.seek(io::SeekFrom::Start(byte_start))?;

        let name_length = file.read_u32::<LittleEndian>()?;
        let mut name = vec![0; name_length as usize];
        file.read_exact(&mut name)?;
        let name = match String::from_utf8(name) {
            Ok(s) => s,
            Err(e) => return Err(Error::new(io::ErrorKind::InvalidData, e)),
        };
        align_stream(&mut file)?;

        if !NON_XML_FILES.into_iter().any(|&n| n == name) {
            // We only want XML files
            let bytes_n = file.read_u32::<LittleEndian>()?;
            let mut xml = vec![0; bytes_n as usize];
            file.read_exact(&mut xml)?;

            let xml = match String::from_utf8(xml) {
                Ok(s) => s,
                Err(e) => return Err(Error::new(io::ErrorKind::InvalidData, e)),
            };
            // println!("{name}");
            fs::write(format!("debug/{name}.xml"), xml)?;
        }

        file.seek(io::SeekFrom::Start(position))?;
    }

    Ok(())
}

fn read_nul_terminated_string<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut res = Vec::new();
    loop {
        let byte = reader.read_u8()?;
        if byte == 0 {
            break;
        }

        res.push(byte);
    }

    let s = match String::from_utf8(res) {
        Ok(s) => s,
        Err(e) => return Err(Error::new(io::ErrorKind::InvalidData, e)),
    };
    Ok(s)
}

fn align_stream<S: Seek + Read>(stream: &mut S) -> io::Result<()> {
    let position = stream.seek(io::SeekFrom::Current(0))?;
    let bytes_to_skip = (4 - (position % 4)) % 4;
    for _ in 0..bytes_to_skip {
        stream.read_u8()?;
    }

    Ok(())
}
