## Introduction

todo!

### Memory map
| NAME         | AREA           | LENGTH       |
| ------------ | -------------- | --------- |
| CLINT        | 0X02000000-->0X02010000 | 0X00010000 |
| PLIC         | 0X0C000000-->0X10000000 | 0X04000000 |
| DRAM         | 0X80000000-->0X88000000 | 0X08000000 |
| UART         | 0XA00003F8-->0XA00003F9 | 0X00000001 |
| RTC          | 0XA0000048-->0XA0000050 | 0X00000008 |
| VGA_CTL      | 0XA0000100-->0XA0000108 | 0X00000008 |
| VGA_FB       | 0XA1000000-->0XA1075300 | 0X00075300 |
| KeyBorad_AM  | 0XA0000060-->0XA0000068 | 0X00000008 |
| Mouse        | 0XA0000070-->0XA0000080 | 0X00000010 |
| Sifive_Uart  | 0XC0000000-->0XC0001000 | 0X00001000 |

## Dependencies
1. sdl2
2. llvm (optional)
## Build

### Build without debug trace
For simplicity,default build are without debug trace,and no need to build `LLVM` static library.
```bash
cargo build --release
```
### Build with debug trace
We use `LLVM` to disassemble instructions, so you need to build `LLVM` static library first.

```bash
todo!
```


## Run
run `CoreMark`
```bash
cargo run --release -- --img ./ready_to_run/coremark-riscv64-nemu.bin
```
run `RT-thread`
```bash
cargo run --release -- --img ./ready_to_run/rtthread.bin
```
run `Linux`
```bash
cargo run release -- --img ./ready_to_run/linux.bin
```
## Test
**test with `riscv-tests`**
```bash
cargo riscv-tests
```
**test with `riscof`**

todo! 

## reference
- [riscv-rust](https://github.com/takahirox/riscv-rust)
- [spike](https://github.com/riscv-software-src/riscv-isa-sim)


