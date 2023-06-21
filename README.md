# Introduction
![run_linux](https://cdn.jsdelivr.net/gh/leesum1/doc/img/leesum1.gif)
RV64emu is a riscv64 emulator written in rust, it is still under development. It can run `CoreMark`,`RT-thread` and `Linux` now. And it is easy to add new devices and support new instructions. The RV64emu is now as a crate, you can use it in your project,and it is also a standalone emulator, you can run it directly.Due to RUST's cross-platform feature, it can run on Linux, Windows and MacOS.Even on the embedded device, such as ESP32 and STM32 which support embeded rust.

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
The simple example of using RV64emu as a crate.You can find it in `examples` directory.

# Standalone emulator

When you build the project, you will get a standalone emulator, you can run it directly.

## Device Map

| NAME         | AREA           | LENGTH       |
| ------------ | -------------- | --------- |
| CLINT        | 0X02000000-->0X02010000 | 0X00010000 |
| PLIC         | 0X0C000000-->0X10000000 | 0X04000000 |
| DRAM         | 0X80000000-->0X88000000 | 0X08000000 |
| UART(AM)         | 0XA00003F8-->0XA00003F9 | 0X00000001 |
| RTC(AM)          | 0XA0000048-->0XA0000050 | 0X00000008 |
| VGA_CTL(AM)      | 0XA0000100-->0XA0000108 | 0X00000008 |
| VGA_FB(AM)       | 0XA1000000-->0XA1075300 | 0X00075300 |
| KeyBorad_AM(AM)  | 0XA0000060-->0XA0000068 | 0X00000008 |
| Mouse(AM)        | 0XA0000070-->0XA0000080 | 0X00000010 |
| Sifive_Uart  | 0XC0000000-->0XC0001000 | 0X00001000 |


## Build

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

```


For simplicity,default build are without debug trace
```bash
cargo build --release
```

# Run
run `CoreMark`
```bash
cargo run --release --features=device_sdl2 -- --img ./ready_to_run/coremark-riscv64-nemu.bin
```
run `RT-thread`
```bash
cargo run --release --features=device_sdl2 -- --img ./ready_to_run/rtthread.bin
```
run `Linux`
```bash
cargo run --release -- --img ./ready_to_run/linux.bin
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

# reference
- [riscv-rust](https://github.com/takahirox/riscv-rust)
- [spike](https://github.com/riscv-software-src/riscv-isa-sim)

