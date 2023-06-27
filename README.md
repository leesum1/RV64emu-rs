# Introduction
![run_linux](https://cdn.jsdelivr.net/gh/leesum1/doc/img/leesum1.gif)

rv64emu is a riscv64 emulator written in rust,It can run `CoreMark`,`RT-thread` and `Linux` now. And it is easy to add new devices and support new instructions. The rv64emu is now as a crate, you can use it in your project,and it is also a standalone emulator, you can run it directly.Due to RUST's cross-platform feature, it can run on Linux, Windows and MacOS.Even on the embedded device, such as ESP32 and STM32 which support embeded rust.

ISA Specification:
- [x] RV64I
- [x] RV64M
- [x] RV64A
- [ ] RV64F
- [ ] RV64D
- [x] RV64C
- [x] MachineMode
- [x] SupervisorMode
- [x] UserMode
- [x] Sv39
- [x] Sv48
- [x] Sv57
- [ ] PMP

# Example
The simplest example of using rv64emu as a crate.You can find it in `examples` directory.

+ **simple_system**  : the simplest example, only have uart and dram
+ **ysyx_am_system** : support AM environment, use ebread to terminate emulation
+ **linux_system** : support linux, you can run linux directly


**features**

|Name|Function|Comment|
|----|--------|-------|
|device_sdl2|support sdl2 device,|such vga and keyboard|
|support_am|support AM environment, use ebread to terminate emulation|[AM github](https://github.com/NJU-ProjectN/abstract-machine) [AM pdf](https://oscpu.github.io/ysyx/events/2021-07-13_AM_Difftest/AM%E8%A3%B8%E6%9C%BA%E8%BF%90%E8%A1%8C%E6%97%B6%E7%8E%AF%E5%A2%83.pdf)|
|caches|support caches,including inst_cache and decode_cache||
|rv_debug_trace|support debug trace,including itrace and ftrace|the log file is in /tmp|
|rv_c|support rv_c extension||
|rv_m|support rv_m extension||
|rv_a|support rv_a extension||


# Run

**Run abstract machine applications**

The following command will run `hello.bin` ,which is a simple application in `ready_to_run` directory. And for details of `ysyx_am_system` example, please refer to `examples/ysyx_am_system.rs`.

```bash
cargo run --release --example=ysyx_am_system --features="std device_sdl2" -- --img ready_to_run/hello.bin
```

Apart from `hello.bin`, there are other applications in `ready_to_run` directory:
- hello.bin
- coremark-riscv64-nemu.bin
- nanos-lite-riscv64-nemu.bin
- rtthread.bin

These applications above were build for `AM` environment.


**Run linux**
```bash
cargo run --release --example=linux_system -- --img ready_to_run/linux.elf
```
# Build linux kernel by yourself
> Please refer to [rv64emu-sdk](https://github.com/leesum1/rv64emu-sdk) for details.

# Test
**test with `riscv-tests`**

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


# reference
- [riscv-rust](https://github.com/takahirox/riscv-rust)
- [spike](https://github.com/riscv-software-src/riscv-isa-sim)
- [abstract-machine](https://github.com/NJU-ProjectN/abstract-machine)

