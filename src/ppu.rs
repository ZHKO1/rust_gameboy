use crate::memory::Memory;
use crate::ppu::FetcherStatus::{GetTile, GetTileDataHigh, GetTileDataLow};
use crate::ppu::PixelType::{Sprite, Window, BG};
use crate::ppu::PpuStatus::{Drawing, HBlank, OAMScan, VBlank};
use crate::util::check_bit;
use std::collections::VecDeque;
use std::{cell::RefCell, rc::Rc};

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

enum Color {
    WHITE = 0xE0F8D0,
    LightGray = 0x88C070,
    DarkGray = 0x346856,
    BlackGray = 0x081820,
}

pub enum PpuStatus {
    OAMScan = 2,
    Drawing = 3,
    HBlank = 0,
    VBlank = 1,
}

#[derive(Clone, Copy, PartialEq)]
enum PixelType {
    BG,
    Window,
    Sprite,
}

#[derive(Clone, Copy)]
struct Pixel {
    ptype: PixelType,
    pcolor: u8,
    palette: bool,
    bg_window_over_obj: bool,
}

enum FetcherStatus {
    GetTile,
    GetTileDataLow,
    GetTileDataHigh,
}

struct Fetcher {
    scan_x: u8,
    scan_y: u8,
    scx: u8,
    scy: u8,
    wx: u8,
    wy: u8,
    cycles: u16,
    oam_x: u8,
    oam_y: u8,
    x_flip: bool,
    y_flip: bool,
    palette: bool,
    bg_window_over_obj: bool,
    ptype: PixelType,
    mmu: Rc<RefCell<dyn Memory>>,
    status: FetcherStatus,
    tile_index: u16,
    tile_data_low: u8,
    tile_data_high: u8,
    buffer: Vec<Pixel>,
}
impl Fetcher {
    fn new(mmu: Rc<RefCell<dyn Memory>>) -> Self {
        Fetcher {
            scan_x: 0,
            scan_y: 0,
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,
            oam_x: 0,
            oam_y: 0,
            x_flip: false,
            y_flip: false,
            palette: false,
            bg_window_over_obj: false,
            ptype: BG,
            mmu,
            cycles: 0,
            status: GetTile,
            tile_index: 0,
            tile_data_low: 0,
            tile_data_high: 0,
            buffer: Vec::new(),
        }
    }
    fn init(&mut self, ptype: PixelType, x: u8, y: u8) {
        self.scan_x = x;
        self.scan_y = y;

        self.scx = 0;
        self.scy = 0;

        self.wx = 0;
        self.wy = 0;

        self.oam_x = 0;
        self.oam_y = 0;

        self.x_flip = false;
        self.y_flip = false;
        self.palette = false;
        self.bg_window_over_obj = false;
        self.ptype = ptype;

        self.cycles = 0;
        self.status = GetTile;

        self.tile_index = 0;
        self.tile_data_low = 0;
        self.tile_data_high = 0;

        self.buffer = Vec::new();
    }
    fn trick(&mut self) {
        if self.cycles == 1 {
            self.cycles = 0;
            return;
        }
        self.cycles += 1;
        match self.status {
            GetTile => {
                self.tile_index = self.get_tile();
                self.status = GetTileDataLow;
            }
            GetTileDataLow => {
                self.tile_data_low = self.get_tile_data_low();
                self.status = GetTileDataHigh;
            }
            GetTileDataHigh => {
                self.tile_data_high = self.get_tile_data_high();
                self.buffer = self.get_buffer();
                self.status = GetTile;
            }
        }
    }
    fn get_tile(&mut self) -> u16 {
        let lcdc = self.mmu.borrow().get(0xFF40);
        let bg_window_tile_area = check_bit(lcdc, 4);
        let bg_tile_map_area = check_bit(lcdc, 3);
        let bg_map_start: u16 = match bg_tile_map_area {
            true => 0x9C00,
            false => 0x9800,
        };
        let window_tile_map_area = check_bit(lcdc, 6);
        let window_map_start: u16 = match window_tile_map_area {
            true => 0x9C00,
            false => 0x9800,
        };

        match self.ptype {
            BG => {
                self.scy = self.mmu.borrow().get(0xFF42);
                self.scx = self.mmu.borrow().get(0xFF43);
                let bg_map_x = (self.scan_x as u16 + self.scx as u16) % 256 / 8;
                let bg_map_y = (self.scan_y as u16 + self.scy as u16) % 256 / 8;
                let bg_map_index = bg_map_x + bg_map_y * 32;
                let bg_map_byte = self.mmu.borrow().get(bg_map_start + bg_map_index);
                let tile_index: u16 = if bg_window_tile_area {
                    0x8000 + bg_map_byte as u16 * 8 * 2
                } else {
                    (0x9000 as i32 + (bg_map_byte as i8) as i32 * 8 * 2) as u16
                };
                tile_index
            }
            Window => {
                self.wy = self.mmu.borrow().get(0xFF4A);
                self.wx = self.mmu.borrow().get(0xFF4B);
                let bg_map_x = (self.scan_x as u16 - (self.wx - 7) as u16) % 256 / 8;
                let bg_map_y = (self.scan_y as u16 - self.wy as u16) % 256 / 8;
                let bg_map_index = bg_map_x + bg_map_y * 32;
                let bg_map_byte = self.mmu.borrow().get(window_map_start + bg_map_index);
                let tile_index: u16 = if bg_window_tile_area {
                    0x8000 + bg_map_byte as u16 * 8 * 2
                } else {
                    (0x9000 as i32 + (bg_map_byte as i8) as i32 * 8 * 2) as u16
                };
                tile_index
            }
            Sprite => self.tile_index,
        }
    }
    fn get_tile_data_low(&self) -> u8 {
        let tile_index = self.tile_index;
        match self.ptype {
            BG => {
                let tile_pixel_y = (self.scan_y as u16 + self.scy as u16) % 8;
                let tile_byte_low = self.mmu.borrow().get(tile_index + tile_pixel_y * 2);
                tile_byte_low
            }
            Window => {
                let tile_pixel_y = (self.scan_y as u16 - self.wy as u16) % 8;
                let tile_byte_low = self.mmu.borrow().get(tile_index + tile_pixel_y * 2);
                tile_byte_low
            }
            Sprite => {
                let mut tile_pixel_y = (self.scan_y as u16 - (self.oam_y - 16) as u16) % 8;
                if self.y_flip {
                    tile_pixel_y = (8 - 1) - tile_pixel_y;
                }
                let tile_byte_low = self.mmu.borrow().get(tile_index + tile_pixel_y * 2);
                tile_byte_low
            }
        }
    }
    fn get_tile_data_high(&self) -> u8 {
        let tile_index = self.tile_index;
        match self.ptype {
            BG => {
                let tile_pixel_y = (self.scan_y as u16 + self.scy as u16) % 8;
                let tile_byte_high = self.mmu.borrow().get(tile_index + tile_pixel_y * 2 + 1);
                tile_byte_high
            }
            Window => {
                let tile_pixel_y = (self.scan_y as u16 - self.wy as u16) % 8;
                let tile_byte_high = self.mmu.borrow().get(tile_index + tile_pixel_y * 2 + 1);
                tile_byte_high
            }
            Sprite => {
                let mut tile_pixel_y = (self.scan_y as u16 - (self.oam_y - 16) as u16) % 8;
                if self.y_flip {
                    tile_pixel_y = (8 - 1) - tile_pixel_y;
                }
                let tile_byte_high = self.mmu.borrow().get(tile_index + tile_pixel_y * 2 + 1);
                tile_byte_high
            }
        }
    }
    fn get_buffer(&mut self) -> Vec<Pixel> {
        let mut result = Vec::new();
        let mut get_pixel_bit: Box<dyn Fn(u8) -> u8> = Box::new(|index: u8| 8 - index - 1);
        let buffer_index_start = match self.ptype {
            BG => (self.scan_x as u16 + self.scx as u16) % 8,
            Window => (self.scan_x as u16 - self.wx as u16) % 8,
            Sprite => {
                if self.x_flip {
                    get_pixel_bit = Box::new(|index: u8| index);
                }
                (self.scan_x as u16 - (self.oam_x - 8) as u16) % 8
            }
        };
        for buffer_index in buffer_index_start..8 {
            let pixel_bit = get_pixel_bit(buffer_index as u8);
            let pixel_low = check_bit(self.tile_data_low, pixel_bit as u8);
            let pixel_high = check_bit(self.tile_data_high, pixel_bit as u8);
            let pvalue = (pixel_low as u8) | ((pixel_high as u8) << 1);
            let pcolor = self.get_color_index(self.ptype, pvalue, self.palette);
            result.push(Pixel {
                ptype: self.ptype,
                pcolor,
                palette: self.palette,
                bg_window_over_obj: self.bg_window_over_obj,
            });
        }
        result
    }
    fn get_color_index(&self, ptype: PixelType, pvalue: u8, is_obp0: bool) -> u8 {
        let palette = match ptype {
            BG | Window => self.mmu.borrow().get(0xFF47),
            Sprite => {
                if is_obp0 {
                    self.mmu.borrow().get(0xFF49)
                } else {
                    self.mmu.borrow().get(0xFF48)
                }
            }
        };
        match pvalue {
            0 => palette & 0b11,
            1 => (palette & 0b1100) >> 2,
            2 => (palette & 0b110000) >> 4,
            3 => (palette & 0b11000000) >> 6,
            _ => {
                panic!("color index is out of range {}", pvalue);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct OAM {
    y: u8,
    x: u8,
    tile_index: u8,
    bg_window_over_obj: bool,
    x_flip: bool,
    y_flip: bool,
    palette: bool,
}
impl OAM {
    fn new(y: u8, x: u8, tile_index: u8, flags: u8) -> Self {
        Self {
            y,
            x,
            tile_index,
            bg_window_over_obj: check_bit(flags, 7),
            x_flip: check_bit(flags, 5),
            y_flip: check_bit(flags, 6),
            palette: check_bit(flags, 4),
        }
    }
    fn is_scaned(&self, ly: u8) -> bool {
        if self.y < 16 {
            return false;
        }
        let y_start = self.y as i32 - 16;
        let y_end = self.y as i32 + 8 - 16;
        ((ly as i32) >= y_start) && ((ly as i32) < y_end) && (self.x != 0)
    }
}

enum FifoTrick {
    BgWindow,
    Sprite,
}
struct FIFO {
    x: u8,
    y: u8,
    status: FifoTrick,
    mmu: Rc<RefCell<dyn Memory>>,
    fetcher: Fetcher,
    queue: VecDeque<Pixel>,
    oam: Vec<OAM>,
}
impl FIFO {
    fn new(mmu: Rc<RefCell<dyn Memory>>) -> Self {
        let fetcher = Fetcher::new(mmu.clone());
        FIFO {
            x: 0,
            y: 0,
            mmu,
            status: FifoTrick::BgWindow,
            fetcher,
            queue: VecDeque::new(),
            oam: vec![],
        }
    }
    fn init(&mut self, y: u8) {
        self.x = 0;
        self.y = y;
        self.queue.clear();
        self.oam.clear();
        self.status = FifoTrick::BgWindow;
        self.fetcher.init(self.get_fetcher_ptype(self.x), self.x, y);
    }
    fn set_oam(&mut self, oam: Vec<OAM>) {
        self.oam = oam;
    }
    fn trick(&mut self) -> Option<Pixel> {
        match self.status {
            FifoTrick::BgWindow => {
                let mut result = None;
                if self.queue.len() > 8 {
                    // 检查当前像素是否上层有Window或Sprite
                    let new_fetch_event =
                        self.check_overlap(self.queue.front().unwrap().ptype, self.x);
                    if let Some(event) = new_fetch_event {
                        match event {
                            Window => {
                                self.queue.clear();
                                self.fetcher.init(Window, self.x, self.y);
                                return None;
                            }
                            Sprite => {
                                self.status = FifoTrick::Sprite;
                                self.fetcher.init(Sprite, self.x, self.y);
                                let oam = self.get_oam(self.x).unwrap();
                                self.fetcher.oam_x = oam.x;
                                self.fetcher.oam_y = oam.y;
                                self.fetcher.x_flip = oam.x_flip;
                                self.fetcher.y_flip = oam.y_flip;
                                self.fetcher.bg_window_over_obj = oam.bg_window_over_obj;
                                self.fetcher.palette = oam.palette;
                                self.fetcher.tile_index = 0x8000 + (oam.tile_index as u16) * 16;
                                return None;
                            }
                            _ => {
                                panic!("ppu fifo trick error");
                            }
                        };
                    }
                    // 执行到这，无异常，正常压入弹出流程
                    self.x += 1;
                    result = self.pop_front();
                }
                if self.fetcher.buffer.len() > 0 {
                    if self.queue.len() <= 8 {
                        for pixel in self.fetcher.buffer.clone().into_iter() {
                            self.push_back(pixel);
                        }
                        let fetcher_x = self.x + self.queue.len() as u8;
                        self.fetcher
                            .init(self.get_fetcher_ptype(fetcher_x), fetcher_x, self.y);
                    }
                } else {
                    self.fetcher.trick();
                }
                result
            }
            FifoTrick::Sprite => {
                if self.fetcher.buffer.len() > 0 {
                    for (index, pixel) in self.fetcher.buffer.iter().enumerate() {
                        if pixel.bg_window_over_obj {
                            let bg_pixel = &self.queue[index];
                            if bg_pixel.pcolor == 0 {
                                self.queue[index] = *pixel;
                            } else {
                                self.queue[index].ptype = Sprite;
                            }
                        } else {
                            if pixel.pcolor != 0 {
                                self.queue[index] = *pixel;
                            } else {
                                self.queue[index].ptype = Sprite;
                            }
                        }
                    }
                    self.status = FifoTrick::BgWindow;
                    let fetcher_x = self.x + self.queue.len() as u8;
                    self.fetcher
                        .init(self.get_fetcher_ptype(fetcher_x), fetcher_x, self.y);
                } else {
                    self.fetcher.trick();
                }
                None
            }
        }
    }
    fn check_overlap(&self, ptype: PixelType, x: u8) -> Option<PixelType> {
        match ptype {
            BG => {
                if self.check_window(x) {
                    Some(Window)
                } else if self.check_sprite(x) {
                    Some(Sprite)
                } else {
                    None
                }
            }
            Window => {
                if self.check_sprite(x) {
                    Some(Sprite)
                } else {
                    None
                }
            }
            Sprite => None,
        }
    }
    fn check_window(&self, x: u8) -> bool {
        let lcdc = self.mmu.borrow().get(0xFF40);
        let window_enable = check_bit(lcdc, 5);
        if !window_enable {
            return false;
        }
        let wy = self.mmu.borrow().get(0xFF4A);
        let wx = self.mmu.borrow().get(0xFF4B);
        (x >= wx - 7) && (self.y > wy)
    }
    fn check_sprite(&self, x: u8) -> bool {
        let lcdc = self.mmu.borrow().get(0xFF40);
        let obj_enable = check_bit(lcdc, 1);
        if !obj_enable {
            return false;
        }
        for oam in self.oam.iter() {
            if x >= oam.x - 8 && x < oam.x {
                return true;
            }
        }
        false
    }
    fn get_oam(&self, x: u8) -> Option<OAM> {
        for oam in self.oam.iter() {
            if x >= oam.x - 8 && x < oam.x {
                return Some(oam.clone());
            }
        }
        None
    }
    fn get_fetcher_ptype(&self, x: u8) -> PixelType {
        if self.check_window(x) {
            Window
        } else {
            BG
        }
    }
    fn push_back(&mut self, pixel: Pixel) {
        self.queue.push_back(pixel);
    }
    fn pop_front(&mut self) -> Option<Pixel> {
        self.queue.pop_front()
    }
    fn clear(&mut self) {
        self.queue.clear();
        self.oam.clear();
    }
}

pub struct PPU {
    cycles: u32,
    status: PpuStatus,
    fifo: FIFO,
    mmu: Rc<RefCell<dyn Memory>>,
    ly_buffer: Vec<u32>,
    pub frame_buffer: [u32; WIDTH * HEIGHT],
}
impl PPU {
    pub fn new(mmu: Rc<RefCell<dyn Memory>>) -> Self {
        let fifo = FIFO::new(mmu.clone());
        let mut ppu = PPU {
            cycles: 0,
            status: OAMScan,
            mmu,
            fifo,
            ly_buffer: Vec::new(),
            frame_buffer: [0; WIDTH * HEIGHT],
        };
        ppu.set_mode(OAMScan);
        ppu
    }
    pub fn trick(&mut self) {
        match self.status {
            OAMScan => {
                if self.cycles == 0 {
                    let ly = self.get_ly();
                    self.fifo.init(ly);
                    let oams = self.oam_scan();
                    self.fifo.set_oam(oams);
                }
                if self.cycles == 79 {
                    self.set_mode(Drawing);
                }
                self.cycles += 1;
            }
            Drawing => {
                let pixel_option = self.fifo.trick();
                if let Some(pixel) = pixel_option {
                    self.ly_buffer.push(self.get_pixel_color(pixel.pcolor));
                    if self.ly_buffer.len() == WIDTH {
                        let ly = self.get_ly();
                        for (scan_x, pixel) in self.ly_buffer.iter().enumerate() {
                            self.frame_buffer[(ly as usize * WIDTH + scan_x) as usize] = *pixel;
                        }
                        self.set_mode(HBlank);
                    }
                } else {
                }
                self.cycles += 1;
            }
            HBlank => {
                let ly = self.get_ly();
                if self.cycles == 455 {
                    if ly == 143 {
                        self.set_mode(VBlank);
                    } else {
                        self.set_mode(OAMScan);
                    }
                    self.set_ly(ly + 1);
                    self.cycles = 0;
                } else {
                    self.cycles += 1;
                }
            }
            VBlank => {
                self.set_vblank_interrupt();
                let ly = self.get_ly();
                if self.cycles == 455 {
                    if ly == 153 {
                        self.set_mode(OAMScan);
                        self.set_ly(0);
                    } else {
                        self.set_ly(ly + 1);
                    }
                    self.cycles = 0;
                } else {
                    self.cycles += 1;
                }
            }
        }
    }
    fn get_pixel_color(&self, color_value: u8) -> u32 {
        match color_value {
            0 => Color::WHITE as u32,
            1 => Color::LightGray as u32,
            2 => Color::DarkGray as u32,
            3 => Color::BlackGray as u32,
            _ => {
                panic!("color_value is out of range {}", color_value);
            }
        }
    }
    fn oam_scan(&self) -> Vec<OAM> {
        let ly = self.get_ly();
        let mut result = vec![];
        for index in 00..40 {
            let oam_address = 0xFE00 + (index as u16) * 4;
            let y = self.mmu.borrow().get(oam_address);
            let x = self.mmu.borrow().get(oam_address + 1);
            let tile_index = self.mmu.borrow().get(oam_address + 2);
            let flags = self.mmu.borrow().get(oam_address + 3);
            let oam = OAM::new(y, x, tile_index, flags);
            if oam.is_scaned(ly) {
                result.push(oam);
            }
            if result.len() == 10 {
                break;
            }
        }
        result
    }
    fn set_ly(&mut self, ly: u8) {
        self.mmu.borrow_mut().set(0xFF44, ly);
    }
    fn get_ly(&self) -> u8 {
        self.mmu.borrow().get(0xFF44)
    }
    fn set_vblank_interrupt(&mut self) {
        let d8 = self.mmu.borrow_mut().get(0xFF0F);
        self.mmu.borrow_mut().set(0xFF0F, d8 | 0x1);
    }
    fn set_mode(&mut self, mode: PpuStatus) {
        let value;
        match mode {
            OAMScan => {
                self.ly_buffer = Vec::new();

                value = 0b10;
            }
            Drawing => {
                value = 0b11;
            }
            HBlank => {
                self.ly_buffer.clear();
                self.fifo.clear();

                value = 0b00;
            }
            VBlank => {
                value = 0b01;
            }
        };
        self.status = mode;
        let d8 = self.mmu.borrow_mut().get(0xFF41);
        let d8 = d8 & 0b11111100 | value;
        self.mmu.borrow_mut().set(0xFF41, d8);
    }
}
