use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub fn u16_from_2u8(low: u8, high: u8) -> u16 {
    u16::from(low) + (u16::from(high) << 8)
}

pub fn u8u8_from_u16(value: u16) -> (u8, u8) {
    let value_low = (value & 0x00ff) as u8;
    let value_high = ((value & 0xff00) >> 8) as u8;
    (value_low, value_high)
}

pub fn check_bit(value: u8, index: u8) -> bool {
    let bit = 1 << index;
    value & bit == bit
}

pub fn read_rom(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let mut rom = vec![];
    let mut file = File::open(path)?;
    file.read_to_end(&mut rom)?;
    Ok(rom)
}

pub fn read_ram(path: impl AsRef<Path>, ram_size: usize) -> Vec<u8> {
    match File::open(path) {
        Ok(mut ok) => {
            let mut ram = Vec::new();
            ok.read_to_end(&mut ram).unwrap();
            ram
        }
        Err(_) => vec![0; ram_size],
    }
}
