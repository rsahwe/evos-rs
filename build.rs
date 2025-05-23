use std::{fs::{self, OpenOptions}, io::{Read, Write}, path::PathBuf};

fn make_static_disk_from_folder<'a>(folder: impl Into<&'a str>) -> Box<[u8]> {
    let folder_name = folder.into();

    let folder = PathBuf::from(folder_name).read_dir().expect(format!("Passed invalid folder {} to make_static_disk_from_folder", folder_name).as_str());

    println!("cargo:rerun-if-changed={}", folder_name);

    let files = folder.map(|file| {
        let name = match file {
            Ok(file) => match file.path().is_dir() {
                true => "evos_fun_impl_no_file".to_string().into(),
                false => file.file_name(),
            },
            Err(err) => panic!("Could not use DirEntry due to {}", err)
        };
        name.clone().into_string().expect(format!("Invalid file name {:?}", name).as_str())
    }).filter(|s| s != "evos_fun_impl_no_file");

    let mut file_count = 0;

    let all = files.map(|file| {
        let mut buf = Vec::new();
        OpenOptions::new().read(true).open(PathBuf::from(folder_name).join(file.as_str())).expect("Could not open file?").read_to_end(&mut buf).expect("Could not read file!");
        file_count += 1;
        (file, buf)
    }).collect::<Vec<_>>();

    assert!(all.len() == file_count);

    let total_len = all.iter().fold(0, |old, (name, content)| old + name.len() + content.len()) + size_of::<usize>() + size_of::<usize>() * file_count * 3;

    let mut end_file = vec![0u8; total_len];

    let mut name_offset = end_file.as_mut_slice().write(&file_count.to_le_bytes()).unwrap();
    let mut offset = name_offset + size_of::<usize>() * file_count * 3;
    for (name, file) in all {
        name_offset += (&mut end_file.as_mut_slice()[name_offset..]).write(&offset.to_le_bytes()).unwrap();
        name_offset += (&mut end_file.as_mut_slice()[name_offset..]).write(&name.len().to_le_bytes()).unwrap();
        name_offset += (&mut end_file.as_mut_slice()[name_offset..]).write(&file.len().to_le_bytes()).unwrap();
        offset += (&mut end_file.as_mut_slice()[offset..]).write(name.as_bytes()).unwrap();
        offset += (&mut end_file.as_mut_slice()[offset..]).write(file.as_slice()).unwrap();
    }

    end_file.into_boxed_slice()
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let kernel = PathBuf::from(std::env::var_os("CARGO_BIN_FILE_EVKRNL_evkrnl").unwrap());

    //TODO: ATTACH KERNEL ID TO KERNEL FILE SYSTEM/PARTITION/DISK

    let file = make_static_disk_from_folder("ramdisk");
    let ramdisk_name = out_dir.join("ramdisk");
    OpenOptions::new().write(true).create(true).open(&ramdisk_name).unwrap().write_all(&file).expect("Could not write ramdisk");

    let uefi_path = out_dir.join("uefi.img");
    bootloader::UefiBoot::new(&kernel).set_ramdisk(&ramdisk_name).create_disk_image(&uefi_path).unwrap();

    let bios_path = out_dir.join("bios.img");
    bootloader::BiosBoot::new(&kernel).set_ramdisk(&ramdisk_name).create_disk_image(&bios_path).unwrap();

    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}