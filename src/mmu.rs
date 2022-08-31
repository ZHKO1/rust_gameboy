use std::path::Path;

use crate::cartridge::{open, Cartridge};
use crate::memory::Memory;
use crate::util::read_rom;
struct MemoryBlock {
    memory: [u8; 0xFFFF - 0x8000 + 1],
    start: u16,
    end: u16,
}

impl MemoryBlock {
    fn new() -> Self {
        let start = 0x8000;
        let end = 0xFFFF;
        MemoryBlock {
            start,
            end,
            memory: [0; 0xFFFF - 0x8000 + 1],
        }
    }
}

impl Memory for MemoryBlock {
    fn get(&self, index: u16) -> u8 {
        if index < self.start || index > self.end {
            return 0xFF;
        }
        self.memory[(index - self.start) as usize]
    }
    fn set(&mut self, index: u16, value: u8) {
        if index < self.start || index > self.end {
            return;
        }
        self.memory[(index - self.start) as usize] = value;
    }
}

pub struct Mmu {
    boot: [u8; 0x100],
    cartridge: Box<dyn Cartridge>,
    other: MemoryBlock,
}

impl Mmu {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let cartridge = open(path);
        let other = MemoryBlock::new();
        let boot_rom = read_rom("tests/DMG_ROM.bin").unwrap();
        let mut boot = [0; 0x100];
        boot.copy_from_slice(&boot_rom[..0x100]);
        Mmu {
            boot,
            cartridge,
            other,
        }
    }
    pub fn is_boot(&self) -> bool {
        let v = self.get(0xFF50);
        v == 0
    }
}
impl Memory for Mmu {
    fn get(&self, index: u16) -> u8 {
        match index {
            0x0000..=0x00ff => {
                if self.is_boot() {
                    self.boot[index as usize]
                } else {
                    self.cartridge.get(index)
                }
            }
            0x0100..=0x7FFF => self.cartridge.get(index),
            0x8000..=0x9FFF => self.other.get(index),
            0xA000..=0xBFFF => self.cartridge.get(index),
            0xC000..=0xFFFF => self.other.get(index),
        }
    }
    fn set(&mut self, index: u16, value: u8) {
        match index {
            0x0000..=0x7FFF => self.cartridge.set(index, value),
            0x8000..=0x9FFF => self.other.set(index, value),
            0xA000..=0xBFFF => self.cartridge.set(index, value),
            0xC000..=0xFFFF => self.other.set(index, value),
        }
    }
}
