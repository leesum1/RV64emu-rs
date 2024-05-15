# Introduction
![run_linux](https://cdn.jsdelivr.net/gh/leesum1/doc/img/leesum1.gif)

RV64emu is a riscv64 emulator written in rust,It can run `CoreMark`,`RT-thread` and `Linux` now. And it is easy to add new devices and support new instructions. The rv64emu is now as a rust crate, you can use it in your project. Due to RUST's cross-platform feature, it can run on Linux, Windows and MacOS.Even on the embedded device, such as `ESP32` and `STM32` which support `Embedded RUST`.

# Features
**ISA Specification:**
- [x] RV64I
- [x] RV64M
- [x] RV64A
- [x] RV64C
- [ ] RV64F
- [ ] RV64D
- [x] MachineMode
- [x] SupervisorMode
- [x] UserMode
- [x] DebugMode
- [x] Sv39
- [x] Sv48
- [x] Sv57
- [ ] PMP

**Caches:**
- [x] InstCache
- [x] DecodeCache
- [x] DataCache (no performance optimization)
- [x] Tlb

**Devices**
- [x] SifiveUart (full support, including interrupt)
- [x] 16550AUart (basic support, no interrupt)
- [x] SifiveClint
- [x] SifivePlic

# Example
The simplest example of using rv64emu as a crate.You can find it in `examples` directory.

+ **simple_system**  : the simplest example, only have uart and ram
+ **ysyx_am_system** : support AM environment, use ebread to terminate emulation
+ **linux_system** : support linux, you can run linux directly
+ **debug_system** : debug module example, you can use gdb to debug the application 


## Run linux
> Build linux kernel by yourself,Please refer to [rv64emu-sdk](https://github.com/leesum1/rv64emu-sdk) for details.
```bash
cargo run --release --example=linux_system -- --img ready_to_run/linux.elf
```

## Debug with GDB
```bash
cargo run --release --example=debug_system -- --img ready_to_run/riscv-tests/elf/rv64ui-p-addiw

openocd -f ready_to_run/rv64emu.cfg
```
connect to openocd with gdb
```bash
riscv64-unknown-elf-gdb ./ready_to_run/riscv-tests/elf/rv64ui-p-addiw

# gdb command
target remote :3333
load
```




# Test
**test with `riscv-tests`**

Soupport running `riscv-tests` in rv64emu out of the box.
There are prebuilt `riscv-tests` in `ready_to_run` directory. You can run it with the following command:
```bash
cargo riscv-tests
```
**test with `riscof`**

todo! 


# No_std support

What a magic feature of rust! You can run `rv64emu` on the embedded device, such as ESP32 and STM32 which support **embeded rust**.

But before build your embeded project, you need to disable some features, such as `device_sdl2`,`rv_debug_trace`.
+ **device_sdl2**: because the embedded device does not support sdl2,and some devices, such as vga and keyboard are based on sdl2.

+ **rv_debug_trace**: because it used `crossbeam-channel` crate,which is not support `no_std`. More over, the embedded device does not support file system,and the log file is too large to store.

+ **caches**: because the embedded device does not have enough memory

## No_std examples
+ [nostd_esp32s3](https://github.com/leesum1/rv64emu_nostd_esp32s3) :A no_std example about how to use rv64emu in esp32s3.


# reference
- [riscv-rust](https://github.com/takahirox/riscv-rust)
- [spike](https://github.com/riscv-software-src/riscv-isa-sim)
- [abstract-machine](https://github.com/NJU-ProjectN/abstract-machine)

