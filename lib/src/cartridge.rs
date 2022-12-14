use crate::memory::Memory;
use chrono::Utc;
use MBC1Mode::{Ram, Rom};

pub fn from_vecu8(rom: Vec<u8>) -> Box<dyn Cartridge> {
    if rom.len() < 0x150 {
        panic!("rom.len()={} < 0x150", rom.len());
    }
    let ram_size = get_ram_size(rom[0x0149 as usize]);
    let cart: Box<dyn Cartridge> = match rom[0x0147 as usize] {
        0x00 => Box::new(RomOnly::new(rom)),
        0x01 => Box::new(MBC1::new(rom, vec![0; ram_size])),
        0x02 => Box::new(MBC1::new(rom, vec![0; ram_size])),
        0x03 => {
            // let ram = read_ram(save_path.clone(), ram_size);
            let ram = vec![0; ram_size];
            Box::new(MBC1::new(rom, ram))
        }
        0x05 => {
            let ram_size = 512;
            Box::new(MBC2::new(rom, vec![0; ram_size]))
        }
        0x06 => {
            let ram_size = 512;
            // let ram = read_ram(save_path.clone(), ram_size);
            let ram = vec![0; ram_size];
            Box::new(MBC2::new(rom, ram))
        }
        0x0F => Box::new(MBC3::new(rom, vec![0; ram_size])),
        0x010 => {
            let ram = vec![0; ram_size];
            Box::new(MBC3::new(rom, ram))
        }
        0x011 => Box::new(MBC3::new(rom, vec![0; ram_size])),
        0x012 => Box::new(MBC3::new(rom, vec![0; ram_size])),
        0x013 => {
            let ram = vec![0; ram_size];
            Box::new(MBC3::new(rom, ram))
        }
        0x019 => Box::new(MBC5::new(rom, vec![0; ram_size])),
        0x01A => Box::new(MBC5::new(rom, vec![0; ram_size])),
        0x01B => {
            let ram = vec![0; ram_size];
            Box::new(MBC5::new(rom, ram))
        }
        _ => panic!("unkown cartridge type"),
    };
    cart
}

fn get_ram_size(code: u8) -> usize {
    let result = match code {
        0x00 => 0,
        0x02 => 8,
        0x03 => 32,
        0x04 => 128,
        0x05 => 64,
        _ => panic!("get_ram_size failed"),
    };
    result * 1024
}

pub trait Cartridge: Stable + Memory {
    fn title(&self) -> String {
        let mut result = String::new();
        let start = 0x0134;
        let end = match self.get(0x0143) {
            0x80 => 0x013E,
            _ => 0x0143,
        };
        for index in start..=end {
            match self.get(index) {
                0 => break,
                v => result.push(v as char),
            }
        }
        result
    }
    fn gbc_flag(&self) -> bool {
        match self.get(0x0143) {
            0x80 | 0xC0 => true,
            _ => false,
        }
    }
    fn get_ram_size(&self) -> usize {
        get_ram_size(self.get(0x0149))
    }
    fn get_cartridge_type(&self) -> &str {
        match self.get(0x0147) {
            0x00 => "ROM ONLY",
            0x01 => "MBC1",
            0x02 => "MBC1+RAM",
            0x03 => "MBC1+RAM+BATTERY",
            0x05 => "MBC2",
            0x06 => "MBC2+BATTERY",
            0x08 => "ROM+RAM 1",
            0x09 => "ROM+RAM+BATTERY 1",
            0x0B => "MMM01",
            0x0C => "MMM01+RAM",
            0x0D => "MMM01+RAM+BATTERY",
            0x0F => "MBC3+TIMER+BATTERY",
            0x10 => "MBC3+TIMER+RAM+BATTERY 2",
            0x11 => "MBC3",
            0x12 => "MBC3+RAM 2",
            0x13 => "MBC3+RAM+BATTERY 2",
            0x19 => "MBC5",
            0x1A => "MBC5+RAM",
            0x1B => "MBC5+RAM+BATTERY",
            0x1C => "MBC5+RUMBLE",
            0x1D => "MBC5+RUMBLE+RAM",
            0x1E => "MBC5+RUMBLE+RAM+BATTERY",
            0x20 => "MBC6",
            0x22 => "MBC7+SENSOR+RUMBLE+RAM+BATTERY",
            0xFC => "POCKET CAMERA",
            0xFD => "BANDAI TAMA5",
            0xFE => "HuC3",
            0xFF => "HuC1+RAM+BATTERY",
            _ => panic!("unkown cartridge type"),
        }
    }
    fn save_status(&self) -> Vec<u8> {
        vec![]
    }
    fn load_status(&mut self, _status: Vec<u8>) {}
}

pub trait Stable {
    fn save_sav(&self) -> Vec<u8> {
        vec![]
    }
    fn load_sav(&mut self, _ram: Vec<u8>) {}
}

pub struct RomOnly {
    rom: Vec<u8>,
}
impl RomOnly {
    fn new(rom: Vec<u8>) -> Self {
        RomOnly { rom }
    }
}
impl Memory for RomOnly {
    fn get(&self, index: u16) -> u8 {
        self.rom[index as usize]
    }
    fn set(&mut self, _: u16, _: u8) {}
}
impl Stable for RomOnly {}
impl Default for RomOnly {
    fn default() -> Self {
        Self { rom: vec![] }
    }
}
impl Cartridge for RomOnly {}

#[derive(serde::Deserialize, serde::Serialize)]
enum MBC1Mode {
    Rom = 0, //  16Mbit ROM/8KByte RAM
    Ram,     // false 4Mbit ROM/32KByte RAM
}
#[derive(serde::Deserialize, serde::Serialize)]
struct MBC1 {
    mode: MBC1Mode,
    #[serde(skip)]
    rom: Vec<u8>,
    #[serde(skip)]
    ram: Vec<u8>,
    max_rom_blank_bit_num: u8,
    rom_blank_bit: u8,
    ram_blank_bit: u8,
    ram_enable: bool,
}
impl MBC1 {
    fn new(rom: Vec<u8>, ram: Vec<u8>) -> Self {
        let len = rom.len();
        let max_rom_blank_bit_num = (len / (4 * 16 * 16 * 16)) as u8;
        MBC1 {
            mode: Rom,
            rom,
            ram,
            max_rom_blank_bit_num,
            rom_blank_bit: 0b00001,
            ram_blank_bit: 0b00,
            ram_enable: false,
        }
    }
    fn get_rom_blank_index(&self) -> u8 {
        let ram_blank_bit = self.ram_blank_bit << 5;
        let result = match self.mode {
            Rom => ram_blank_bit | (self.rom_blank_bit & 0x1f),
            Ram => self.rom_blank_bit,
        };
        match result {
            0x00 => 0x01,
            0x20 => 0x21,
            0x40 => 0x41,
            0x60 => 0x61,
            _ => result,
        }
    }
    fn get_ram_blank_index(&self) -> u8 {
        match self.mode {
            Rom => 0,
            Ram => self.ram_blank_bit,
        }
    }
}
impl Memory for MBC1 {
    fn get(&self, index: u16) -> u8 {
        let mut rom_blank_index = self.get_rom_blank_index();
        let ram_blank_index = self.get_ram_blank_index();
        match index {
            0..=0x3FFF => self.rom[index as usize],
            0x4000..=0x7FFF => {
                rom_blank_index = rom_blank_index & ((self.max_rom_blank_bit_num - 1) as u8);
                let rom_index =
                    rom_blank_index as usize * 0x4000 as usize + (index - 0x4000) as usize;
                self.rom[rom_index]
            }
            0xA000..=0xBFFF => {
                if self.ram_enable && self.ram.len() > 0 {
                    let ram_index =
                        ram_blank_index as usize * 0x2000 as usize + (index - 0xA000) as usize;
                    self.ram[ram_index]
                } else {
                    0xFF
                }
            }
            _ => panic!("out range of MC1"),
        }
    }
    fn set(&mut self, index: u16, value: u8) {
        match index {
            0x0000..=0x1FFF => {
                if value & 0x0F == 0x0A {
                    self.ram_enable = true;
                } else {
                    self.ram_enable = false;
                }
            }
            0x2000..=0x3FFF => {
                self.rom_blank_bit = value & 0x1F;
            }
            0x4000..=0x5FFF => {
                self.ram_blank_bit = value & 0x03;
            }
            0x6000..=0x7FFF => match value {
                0x00 => self.mode = Rom,
                0x01 => self.mode = Ram,
                _ => {}
            },
            0xA000..=0xBFFF => {
                if self.ram_enable && self.ram.len() > 0 {
                    let ram_blank_index = self.get_ram_blank_index();
                    let ram_index =
                        ram_blank_index as usize * 0x2000 as usize + (index - 0xA000) as usize;
                    self.ram[ram_index] = value;
                } else {
                }
            }
            _ => panic!("out range of MC1"),
        }
    }
}
impl Stable for MBC1 {
    fn save_sav(&self) -> Vec<u8> {
        self.ram.clone()
    }
    fn load_sav(&mut self, ram: Vec<u8>) {
        self.ram = ram;
    }
}
impl Cartridge for MBC1 {
    fn save_status(&self) -> Vec<u8> {
        let mut data = Vec::new();
        bincode::serialize_into(&mut data, &self).unwrap();
        data
    }
    fn load_status(&mut self, _status: Vec<u8>) {
        let result: Self = bincode::deserialize_from(_status.as_slice()).unwrap();
        self.mode = result.mode;
        self.max_rom_blank_bit_num = result.max_rom_blank_bit_num;
        self.rom_blank_bit = result.rom_blank_bit;
        self.ram_blank_bit = result.ram_blank_bit;
        self.ram_enable = result.ram_enable;
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct MBC2 {
    #[serde(skip)]
    rom: Vec<u8>,
    #[serde(skip)]
    ram: Vec<u8>,
    rom_blank: u8,
    ram_enable: bool,
    max_rom_blank_bit_num: u8,
}
impl MBC2 {
    fn new(rom: Vec<u8>, ram: Vec<u8>) -> Self {
        let len = rom.len();
        let max_rom_blank_bit_num = (len / (4 * 16 * 16 * 16)) as u8;
        MBC2 {
            rom,
            ram,
            rom_blank: 1,
            ram_enable: false,
            max_rom_blank_bit_num,
        }
    }
}
impl Memory for MBC2 {
    fn get(&self, index: u16) -> u8 {
        match index {
            0..=0x3FFF => self.rom[index as usize],
            0x4000..=0x7FFF => {
                let rom_blank = self.rom_blank & ((self.max_rom_blank_bit_num - 1) as u8);
                let rom_index = rom_blank as usize * 0x4000 as usize + (index - 0x4000) as usize;
                self.rom[rom_index]
            }
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let ram_index = (index - 0xA000) % 0x0200;
                    self.ram[ram_index as usize] | 0xF0
                } else {
                    0xFF
                }
            }
            _ => panic!("out range of MC2"),
        }
    }
    fn set(&mut self, index: u16, value: u8) {
        match index {
            0x0000..=0x3FFF => {
                let bit = ((index & 0x100) >> 8) & 0b1;
                if bit == 0 {
                    if value & 0x0F == 0x0A {
                        self.ram_enable = true;
                    } else {
                        self.ram_enable = false;
                    }
                } else {
                    if value != 0 {
                        self.rom_blank = value & 0x0F;
                        if self.rom_blank == 0 {
                            self.rom_blank = 1;
                        }
                    }
                }
            }
            0x4000..=0x7FFF => {}
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let ram_index = (index - 0xA000) % 0x0200;
                    self.ram[ram_index as usize] = value;
                }
            }
            _ => panic!("out range of MC2"),
        }
    }
}
impl Stable for MBC2 {
    fn save_sav(&self) -> Vec<u8> {
        self.ram.clone()
    }
    fn load_sav(&mut self, ram: Vec<u8>) {
        self.ram = ram;
    }
}
impl Cartridge for MBC2 {
    fn save_status(&self) -> Vec<u8> {
        let mut data = Vec::new();
        bincode::serialize_into(&mut data, &self).unwrap();
        data
    }
    fn load_status(&mut self, _status: Vec<u8>) {
        let result: Self = bincode::deserialize_from(_status.as_slice()).unwrap();
        self.rom_blank = result.rom_blank;
        self.ram_enable = result.ram_enable;
        self.max_rom_blank_bit_num = result.max_rom_blank_bit_num;
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct MBC3RTC {
    s: u8,
    m: u8,
    h: u8,
    dl: u8,
    dh: u8,
    zero: u64,
}
impl MBC3RTC {
    fn new() -> Self {
        let zero = Utc::now().timestamp() as u64;
        Self {
            s: 0,
            m: 0,
            h: 0,
            dl: 0,
            dh: 0,
            zero,
        }
    }
    fn latch_clock(&mut self) {
        let time = Utc::now().timestamp() as u64;
        let duration = time - self.zero;
        self.s = (duration % 60) as u8;
        self.m = ((duration / 60) % 60) as u8;
        self.h = ((duration / 3600) % 24) as u8;
        let day = duration / (3600 * 24);
        self.dl = (day % 256) as u8;
        match day {
            0x0000..=0x00FF => {}
            0x0100..=0x01FF => {
                self.dh |= 1;
            }
            _ => {
                self.dh |= 1;
                self.dh |= 8;
            }
        };
    }
}
impl Memory for MBC3RTC {
    fn get(&self, index: u16) -> u8 {
        match index {
            0x08 => self.s,
            0x09 => self.m,
            0x0A => self.h,
            0x0B => self.dl,
            0x0C => self.dh,
            _ => panic!("MBC3RTC get out of range {:04x}", index),
        }
    }
    fn set(&mut self, index: u16, value: u8) {
        match index {
            0x08 => self.s = value,
            0x09 => self.m = value,
            0x0A => self.h = value,
            0x0B => self.dl = value,
            0x0C => self.dh = value,
            _ => panic!("MBC3RTC set out of range {:04x}", index),
        }
    }
}
impl Stable for MBC3RTC {
    fn save_sav(&self) -> Vec<u8> {
        self.zero.to_be_bytes().to_vec().clone()
    }
    fn load_sav(&mut self, ram: Vec<u8>) {
        let mut tmp: [u8; 8] = [0; 8];
        tmp.copy_from_slice(&ram);
        self.zero = u64::from_be_bytes(tmp);
    }
}
#[derive(serde::Deserialize, serde::Serialize)]
struct MBC3 {
    rtc: MBC3RTC,
    #[serde(skip)]
    rom: Vec<u8>,
    #[serde(skip)]
    ram: Vec<u8>,
    rom_blank: u8,
    ram_blank: u8,
    ram_enable: bool,
    last_write_value: u8,
}
impl MBC3 {
    fn new(rom: Vec<u8>, ram: Vec<u8>) -> Self {
        MBC3 {
            rtc: MBC3RTC::new(),
            rom,
            ram,
            rom_blank: 1,
            ram_blank: 0,
            ram_enable: false,
            last_write_value: 0x01,
        }
    }
}
impl Memory for MBC3 {
    fn get(&self, index: u16) -> u8 {
        match index {
            0..=0x3FFF => self.rom[index as usize],
            0x4000..=0x7FFF => {
                let rom_index =
                    self.rom_blank as usize * 0x4000 as usize + (index - 0x4000) as usize;
                self.rom[rom_index]
            }
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    if self.ram_blank <= 0x03 {
                        let ram_index =
                            self.ram_blank as usize * 0x2000 as usize + (index - 0xA000) as usize;
                        self.ram[ram_index]
                    } else {
                        self.rtc.get(self.ram_blank as u16)
                    }
                } else {
                    0x00
                }
            }
            _ => panic!("out range of MC3"),
        }
    }
    fn set(&mut self, index: u16, value: u8) {
        match index {
            0x0000..=0x1FFF => {
                if value & 0x0F == 0x0A {
                    self.ram_enable = true;
                } else {
                    self.ram_enable = false;
                }
            }
            0x2000..=0x3FFF => {
                self.rom_blank = value & 0x7F;
                if self.rom_blank == 0x00 {
                    self.rom_blank = 0x01;
                }
            }
            0x4000..=0x5FFF => {
                self.ram_blank = value;
            }
            0x6000..=0x7FFF => {
                if self.last_write_value == 0x00 && value == 0x01 {
                    self.rtc.latch_clock();
                }
                self.last_write_value = value;
            }
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    if self.ram_blank <= 0x03 {
                        let ram_blank_index = self.ram_blank;
                        let ram_index =
                            ram_blank_index as usize * 0x2000 as usize + (index - 0xA000) as usize;
                        self.ram[ram_index] = value;
                    } else {
                        self.rtc.set(self.ram_blank as u16, value);
                    }
                }
            }
            _ => panic!("out range of MC3"),
        }
    }
}
impl Stable for MBC3 {
    fn save_sav(&self) -> Vec<u8> {
        let rtc = self.rtc.save_sav();
        [rtc, self.ram.clone()].concat()
    }
    fn load_sav(&mut self, ram: Vec<u8>) {
        let mut rtc: Vec<u8> = vec![];
        for i in 0..=7 {
            rtc.push(ram[i]);
        }
        self.rtc.load_sav(rtc);
        self.ram = vec![];
        for i in 7..ram.len() {
            self.ram.push(ram[i]);
        }
    }
}
impl Cartridge for MBC3 {
    fn save_status(&self) -> Vec<u8> {
        let mut data = Vec::new();
        bincode::serialize_into(&mut data, &self).unwrap();
        data
    }
    fn load_status(&mut self, _status: Vec<u8>) {
        let result: Self = bincode::deserialize_from(_status.as_slice()).unwrap();
        self.rtc = result.rtc;
        self.rom_blank = result.rom_blank;
        self.ram_blank = result.ram_blank;
        self.ram_enable = result.ram_enable;
        self.last_write_value = result.last_write_value;
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct MBC5 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_blank_low_bit: u8,
    rom_blank_high_bit: u8,
    ram_blank: u8,
    ram_enable: bool,
    max_rom_blank_bit_num: usize,
}
impl MBC5 {
    fn new(rom: Vec<u8>, ram: Vec<u8>) -> Self {
        let len = rom.len();
        let max_rom_blank_bit_num = len / (4 * 16 * 16 * 16);
        Self {
            rom,
            ram,
            rom_blank_low_bit: 0x01,
            rom_blank_high_bit: 0x0,
            ram_blank: 0,
            ram_enable: false,
            max_rom_blank_bit_num,
        }
    }
    fn get_rom_blank_index(&self) -> usize {
        ((self.rom_blank_high_bit as usize) << 8) | self.rom_blank_low_bit as usize
    }
}
impl Memory for MBC5 {
    fn get(&self, index: u16) -> u8 {
        match index {
            0..=0x3FFF => self.rom[index as usize],
            0x4000..=0x7FFF => {
                let rom_blank_index = self.get_rom_blank_index();
                let rom_blank_index = rom_blank_index & (self.max_rom_blank_bit_num - 1);
                let rom_index =
                    rom_blank_index as usize * 0x4000 as usize + (index - 0x4000) as usize;
                self.rom[rom_index]
            }
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let ram_index =
                        self.ram_blank as usize * 0x2000 as usize + (index - 0xA000) as usize;
                    self.ram[ram_index]
                } else {
                    0x00
                }
            }
            _ => panic!("out range of MC5"),
        }
    }
    fn set(&mut self, index: u16, value: u8) {
        match index {
            0x0000..=0x1FFF => {
                if value & 0x0F == 0x0A {
                    self.ram_enable = true;
                } else {
                    self.ram_enable = false;
                }
            }
            0x2000..=0x2FFF => {
                self.rom_blank_low_bit = value;
            }
            0x3000..=0x3FFF => {
                self.rom_blank_high_bit = value & 0x01;
            }
            0x4000..=0x5FFF => {
                self.ram_blank = value;
            }
            0x6000..=0x7FFF => {}
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let ram_blank_index = self.ram_blank;
                    let ram_index =
                        ram_blank_index as usize * 0x2000 as usize + (index - 0xA000) as usize;
                    self.ram[ram_index] = value;
                }
            }
            _ => panic!("out range of MC5"),
        }
    }
}
impl Stable for MBC5 {
    fn save_sav(&self) -> Vec<u8> {
        self.ram.clone()
    }
    fn load_sav(&mut self, ram: Vec<u8>) {
        self.ram = ram;
    }
}
impl Cartridge for MBC5 {
    fn save_status(&self) -> Vec<u8> {
        let mut data = Vec::new();
        bincode::serialize_into(&mut data, &self).unwrap();
        data
    }
    fn load_status(&mut self, _status: Vec<u8>) {
        let result: Self = bincode::deserialize_from(_status.as_slice()).unwrap();
        self.rom_blank_low_bit = result.rom_blank_low_bit;
        self.rom_blank_high_bit = result.rom_blank_high_bit;
        self.ram_blank = result.ram_blank;
        self.ram_enable = result.ram_enable;
        self.max_rom_blank_bit_num = result.max_rom_blank_bit_num;
    }
}
