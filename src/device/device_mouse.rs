use ring_channel::RingReceiver;

use device_trait::DeviceBase;

use super::device_trait;

const MOUSE_KEY_OFFSET: u64 = 0;
const POSITION_X_OFFSET: u64 = 8;
const POSITION_Y_OFFSET: u64 = 12;

pub struct DeviceMouseItem {
    pub mouse_btn_state: u32,
    pub x: u32,
    pub y: u32,
}

impl DeviceMouseItem {
    fn new() -> Self {
        DeviceMouseItem {
            mouse_btn_state: 0,
            x: 0,
            y: 0,
        }
    }

    fn get_positon(&self) -> u64 {
        ((self.x as u64) << 32) | (self.y as u64)
    }
}

pub struct DeviceMouse {
    rx_mouse: RingReceiver<DeviceMouseItem>,
    mouse_state: DeviceMouseItem,
}

impl DeviceMouse {
    pub fn new(rx_mouse: RingReceiver<DeviceMouseItem>) -> Self {
        DeviceMouse {
            rx_mouse,
            mouse_state: DeviceMouseItem::new(),
        }
    }
}

impl DeviceBase for DeviceMouse {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {

        if let Ok(item) = self.rx_mouse.try_recv() {
            self.mouse_state = item
        }

        match (addr, len) {
            (MOUSE_KEY_OFFSET, _) => self.mouse_state.mouse_btn_state as u64,
            (POSITION_X_OFFSET, _) => self.mouse_state.x as u64,
            (POSITION_Y_OFFSET, _) => self.mouse_state.y as u64,
            (addr, len) => panic!("DeviceMouse: addr:{addr},len:{len}"),
        }
    }

    fn do_write(&mut self, _addr: u64, _data: u64, _len: usize) -> u64 {
        panic!("DeviceMouse should not wrtie")
    }

    fn do_update(&mut self) {
        // if let Ok(item) = self.rx_mouse.try_recv() {
        //     self.mouse_state = item
        // }
    }

    fn get_name(&self) -> &'static str {
        "Mouse"
    }
}
