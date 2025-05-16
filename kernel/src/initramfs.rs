use spin::RwLock;

use crate::debug;

pub struct InitRamFs {
    raw: Option<&'static [u8]>,
}

static INITRAMFS: RwLock<InitRamFs> = RwLock::new(InitRamFs { raw: None });

pub(crate) fn init(ramdisk_location: u64, ramdisk_len: u64) {
    // SAFETY: GUARANTEED BY BOOTLOADER
    let file_slice = unsafe { core::slice::from_raw_parts(ramdisk_location as *const u8, ramdisk_len as usize) };

    INITRAMFS.write().raw = Some(file_slice);

    debug!("InitRamFs contents:");

    for (file_name, file_content) in InitRamFs::iter() {
        debug!("    File `{}` with size 0x{:016x} bytes", file_name, file_content.len());
    }
}

impl InitRamFs {
    pub fn open_file(name: &str) -> Option<&'static [u8]> {
        Self::iter().find_map(|(file, content)| (file == name).then(|| content))
    }

    pub fn iter() -> InitRamFileIterator {
        let mut file_count = [0; 8];
        file_count.copy_from_slice(&INITRAMFS.read().raw.unwrap()[0..size_of::<usize>()]);
        let file_count = usize::from_le_bytes(file_count);

        InitRamFileIterator { raw: INITRAMFS.read().raw.unwrap(), file_count, current_file: 0 }
    }
}

pub struct InitRamFileIterator {
    raw: &'static [u8],
    file_count: usize,
    current_file: usize,
}

impl Iterator for InitRamFileIterator {
    type Item = (&'static str, &'static [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_file >= self.file_count {
            None
        } else {
            let table_slice = &self.raw[8..];
            let current_slice = &table_slice[3 * 8 * self.current_file..3 * 8 * (self.current_file + 1)];

            let mut buffer = [0; 8];
            buffer.copy_from_slice(&current_slice[0..8]);
            let name_offset = usize::from_le_bytes(buffer);
            buffer.copy_from_slice(&current_slice[8..8 * 2]);
            let name_len = usize::from_le_bytes(buffer);
            let file_offset = name_offset + name_len;
            buffer.copy_from_slice(&current_slice[8 * 2..8 * 3]);
            let file_len = usize::from_le_bytes(buffer);

            self.current_file += 1;

            Some((str::from_utf8(&self.raw[name_offset..name_offset + name_len]).expect("InitRamFs file name invalid!!!"), &self.raw[file_offset..file_offset + file_len]))
        }
    }
}
