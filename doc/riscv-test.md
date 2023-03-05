# Riscv-test 

## rv64-mi

> rv64 Mathine privilege mode tests

```makefile
rv64mi_sc_tests = \
	access \
	breakpoint \
	csr \
	mcsr \
	illegal \
	ma_fetch \
	ma_addr \
	scall \
	sbreak \
	ld-misaligned \
	lw-misaligned \
	lh-misaligned \
	sh-misaligned \
	sw-misaligned \
	sd-misaligned \
	zicntr \
```

* [ ] access

currently, does not support PMP
* [ ] breakpoint

currently, does not support breakpoint
* [ ] csr
* [ ] mcsr
* [ ] illegal
* [x] ma_fetch
* [x] ma_addr
* [x] scall
* [ ] sbreak
* [x] ld-misaligned
* [x] lw-misaligned
* [x] lh-misaligned
* [x] sh-misaligned
* [x] sw-misaligned
* [x] sd-misaligned
* [ ] zicntr

## rv64-si

> rv64 Supervisor privilege mode tests
```c
rv64si_sc_tests = \
	csr \
	dirty \
	icache-alias \
	ma_fetch \
	scall \
	wfi \
	sbreak \
```

* [ ] csr
* [ ] dirty
* [ ] icache-alias
* [x] ma_fetch
* [x] scall
* [x] wfi
* [ ] sbreak
