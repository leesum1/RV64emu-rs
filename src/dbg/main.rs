use std::{cell::RefCell, rc::Rc};

mod debug_module;
mod debug_module_new;
mod debug_module_register;
mod dm_interface;
mod jtag_driver;
mod jtag_state;
mod remote_bitbang;
mod test_core;

fn main() {
    let _ = env_logger::try_init_from_env(env_logger::Env::default().default_filter_or("debug"));

    // let dm = debug_module::DebugModule::new();
    // let mut jtag_driver = jtag_driver::JtagDriver::new(dm);
    let mut remote_bitbang = remote_bitbang::RemoteBitBang::new("0.0.0.0", 23456);

    let hart0 = Rc::new(RefCell::new(test_core::TestCore::new()));
    let dm_new = debug_module_new::DebugModule::new(hart0.clone());
    let mut jtag_driver_new = jtag_driver::JtagDriver::new(dm_new);

    loop {
        remote_bitbang.tick(&mut jtag_driver_new);
        hart0.borrow_mut().tick();

        // remote_bitbang.tick(&mut jtag_driver);
    }
}
