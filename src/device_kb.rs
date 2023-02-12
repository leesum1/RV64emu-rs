use std::{
    io::{self, Write},
    sync::mpsc::Receiver,
};

use ring_channel::RingReceiver;
use sdl2::keyboard::Scancode;

use crate::device_trait::DeviceBase;

// int keymap[256] = { 0,0,0,0,43,60,58,45,31,46,47,48,36,49,50,51,62,61,37,38,
//     29,32,44,33,35,59,30,57,34,56,15,16,17,18,19,20,21,22,23,
//     24,54,1,27,28,70,25,26,39,40,41,0,52,53,14,63,64,65,42,2,
//     3,4,5,6,7,8,9,10,11,12,13,0,0,0,77,79,81,78,80,82,76,75,74,
//     73,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,68,0,0,0,0,0,0,0,0,0,
//     0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
//     0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
//     0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
//     0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,67,55,69,0,72,
//     66,71,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0 };

const AM_KEYMAP: [u32; 256] = [
    0, 0, 0, 0, 43, 60, 58, 45, 31, 46, 47, 48, 36, 49, 50, 51, 62, 61, 37, 38, 29, 32, 44, 33, 35,
    59, 30, 57, 34, 56, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 54, 1, 27, 28, 70, 25, 26, 39, 40,
    41, 0, 52, 53, 14, 63, 64, 65, 42, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 0, 0, 0, 77, 79, 81,
    78, 80, 82, 76, 75, 74, 73, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 68, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 67, 55, 69, 0, 72, 66, 71,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const KEYDOWN_MASK: u32 = 0x8000;

pub struct DeviceKbItem {
    pub scancode: Scancode,
    pub is_keydown: bool,
}

impl DeviceKbItem {
    pub fn get_am_keycode(&self) -> u32 {
        let am_code = AM_KEYMAP[self.scancode as usize];
        let mask = match self.is_keydown {
            true => KEYDOWN_MASK,
            false => 0,
        };

        if am_code != 0 {
            am_code | mask
        } else {
            0
        }
    }
}

pub struct DeviceKB {
    rx: RingReceiver<DeviceKbItem>,
}

impl DeviceKB {
    pub fn new(rx: RingReceiver<DeviceKbItem>) -> Self {
        DeviceKB { rx }
    }

    pub fn get_am_key(&mut self) -> u32 {
        self.rx.try_recv().map_or(0, |item| item.get_am_keycode())
    }
}

impl DeviceBase for DeviceKB {
    fn do_read(&mut self, _addr: u64, _len: usize) -> u64 {
        self.get_am_key() as u64
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        panic!("deviceKB should not wrtie")
    }

    fn get_name(& self) -> &'static str {
        "KeyBorad_AM"
    }
}
