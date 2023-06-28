use std::{cell::Cell, rc::Rc};

use sdl2::{pixels::PixelFormatEnum, render::WindowCanvas};

use crate::device::device_trait::DeviceBase;

const VGA_H: usize = 300;
const VGA_W: usize = 400;
const VGA_BUF_SIZE: usize = VGA_H * VGA_W * 4;
// type VgactlRx = Consumer<bool, Rc<LocalRb<bool, Vec<MaybeUninit<bool>>>>>;
type VgactlRx = Rc<Cell<bool>>;

pub struct DeviceVGA {
    pub vga_canvas: WindowCanvas,
    pub pix_buf: Box<[u8]>,
    rx: VgactlRx,
}

impl DeviceVGA {
    pub fn new(canvas_w: WindowCanvas, rx: VgactlRx) -> Self {
        let buff = vec![0_u8; VGA_BUF_SIZE].into_boxed_slice();

        DeviceVGA {
            vga_canvas: canvas_w,
            pix_buf: buff,
            rx,
        }
    }

    pub fn get_size() -> usize {
        VGA_BUF_SIZE
    }

    pub fn updata_vga(&mut self) {
        let texture_creator = self.vga_canvas.texture_creator();

        let mut texture = texture_creator
            .create_texture_target(PixelFormatEnum::ARGB8888, VGA_W as u32, VGA_H as u32)
            .map_err(|e| e.to_string())
            .unwrap();

        texture.update(None, &self.pix_buf, 4 * VGA_W).unwrap();

        self.vga_canvas.copy(&texture, None, None).unwrap();
        self.vga_canvas.present();
    }
}

impl DeviceBase for DeviceVGA {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        let mut data_bytes = 0_u64.to_le_bytes();
        data_bytes[..(len)].copy_from_slice(&self.pix_buf[(addr as usize)..(addr as usize + len)]);
        u64::from_le_bytes(data_bytes)
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        let data_bytes = data.to_le_bytes();
        self.pix_buf[(addr as usize)..(addr as usize + len)].copy_from_slice(&data_bytes[..(len)]);

        data
    }

    fn do_update(&mut self) {
        // if self.rx.pop().is_some() {
        //     self.updata_vga();
        // }

        if self.rx.get() {
            self.updata_vga();
            self.rx.set(false);
        }
    }

    fn get_name(&self) -> &'static str {
        "AM_VGA_FB"
    }
}

#[cfg(test)]
mod test_vga {

    use std::thread;

    fn _test2() {
        //https://github.com/Rust-SDL2/rust-sdl2/issues/1063
        let sdl2 = sdl2::init().expect("failed to initialize SDL2");
        let _timer_system = sdl2.timer().expect("failed to initialize timer subsystem");
        let event_system = sdl2.event().expect("fail");

        // Contrived, but definitely safe.
        // Another way to get a 'static subsystem reference is through the use of a global,
        // i.e. via `lazy_static`. This is also the *only* way to actually use a subsystem on another thread.
        let static_timer: &'static _ = Box::leak(Box::new(event_system));

        let _a = thread::spawn(move || {
            // Now that we have sent our TimerSubsystem to another thread...
            let timer = static_timer;

            // ...we now have an Sdl context on two threads.
            let _smuggled_sdl = timer.sdl();
            let _evet_dmp = _smuggled_sdl.event_pump();
        });

        _a.join().unwrap();
    }
}
