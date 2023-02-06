use std::slice::Windows;

use sdl2::{
    libc::SYS_set_thread_area,
    pixels::PixelFormatEnum,
    render::{Canvas, Texture, TextureCreator, WindowCanvas},
    surface::Surface,
    video::{Window, WindowContext},
    Sdl, VideoSubsystem,
};

use crate::device_trait::DeviceBase;

const VGA_H: usize = 300;
const VGA_W: usize = 400;
const VGA_BUF_SIZE: usize = VGA_H * VGA_W * 4;

pub struct DeviceVGA {
    pub vga_canvas: WindowCanvas,
    pub pix_buf: [u8; VGA_BUF_SIZE],
    pub count: usize,
}

impl DeviceVGA {
    pub fn new(window: Window) -> Self {
        let canvas = window.into_canvas().build().unwrap();

        DeviceVGA {
            vga_canvas: canvas,
            pix_buf: [0; VGA_BUF_SIZE],
            count: 0,
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

    pub fn init(&mut self) -> Result<(), String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem
            .window("rust-sdl2 demo: Video", 800, 600)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let mut canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())?;

        let texture_creator = canvas.texture_creator();

        let x = texture_creator
            .create_texture_streaming(PixelFormatEnum::ARGB8888, 400, 300)
            .map_err(|e| e.to_string())?;

        Ok(())
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
        self.count += 1;

        if self.count == VGA_H * VGA_W {
            self.count = 0;
            self.updata_vga();
        }
        data
    }
}

#[cfg(test)]
mod test_vga {

    use std::{
        rc::Rc,
        sync::{mpsc, Arc},
        thread,
    };

    use chrono::Duration;
    use sdl2::{event::Event, keyboard::Keycode};

    use crate::device_trait::DeviceBase;

    use super::{DeviceVGA, VGA_BUF_SIZE};

    #[test]
    fn vga_write() {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window("rust-sdl2 demo: Video", 800, 600)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let mut test1 = DeviceVGA::new(window);

        let event_system = sdl_context.event().expect("fail");

        // Contrived, but definitely safe.
        // Another way to get a 'static subsystem reference is through the use of a global,
        // i.e. via `lazy_static`. This is also the *only* way to actually use a subsystem on another thread.
        let static_event: &'static _ = Box::leak(Box::new(event_system));

        // 创建消息通道，tx是生产者，rx是消费者
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            // 在新线程中向主线程发送消息，send返回Result<T>类型，
            // 这里简单使用unwrap，遇到错误时将抛出panic!
            let _event_system = static_event;
            let _sdl_context = _event_system.sdl();
            let mut event_pump = _sdl_context.event_pump().expect("fail to get event_pump");
            loop {
                for event in event_pump.poll_iter() {
                    tx.send(event).unwrap();
                    // match event {
                    //     Event::Quit { .. }
                    //     | Event::KeyDown {
                    //         keycode: Some(Keycode::Escape),
                    //         ..
                    //     } => break 'running,
                    //     // skip mouse motion intentionally because of the verbose it might cause.
                    //     Event::MouseMotion { .. } => {}
                    //     e => {
                    //         println!("{:?}", e);
                    //     }
                    // }
                }
                // The rest of the game loop goes here...
            }
        });

        loop {
            for i in (0..VGA_BUF_SIZE).step_by(4) {
                test1.do_write(i as u64, (i * 100) as u64, 4);
            }
            let received = rx.recv().unwrap();

            match received {
                // skip mouse motion intentionally because of the verbose it might cause.
                Event::MouseMotion { .. } => {}
                // e => {
                //     println!("{:?}", e);
                // }
                Event::KeyUp {
                    timestamp,
                    window_id,
                    keycode,
                    scancode,
                    keymod,
                    repeat,
                } => {

                    
                }
                Event::KeyDown {
                    timestamp,
                    window_id,
                    keycode,
                    scancode,
                    keymod,
                    repeat,
                } => {}

                _ => todo!(),
            }

            test1.updata_vga();
        }
    }

    fn test2() {
        //https://github.com/Rust-SDL2/rust-sdl2/issues/1063
        let sdl2 = sdl2::init().expect("failed to initialize SDL2");
        let timer_system = sdl2.timer().expect("failed to initialize timer subsystem");
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
            let evet_dmp = _smuggled_sdl.event_pump();
        });

        _a.join();
    }
}
