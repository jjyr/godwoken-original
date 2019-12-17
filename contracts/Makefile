# C compile
C_TARGET := riscv64-unknown-elf
CC := $(C_TARGET)-gcc
LD := $(C_TARGET)-gcc
CFLAGS := -O3 -I c/ckb-c-stdlib -Wall -Werror -Wno-nonnull -Wno-unused-function
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections -Wl,-s

# rust compile
RUST_TARGET := riscv64imac-unknown-none-elf

# CKB binary patcher 
install-binary-patcher:
	which ckb-binary-patcher || \
		cargo install --force --git https://github.com/xxuejie/ckb-binary-patcher.git
binary-patch: install-binary-patcher
	ckb-binary-patcher -i binaries/main -o binaries/main

# docker pull nervos/ckb-riscv-gnu-toolchain:bionic-20190702
BUILDER_DOCKER := nervos/ckb-riscv-gnu-toolchain@sha256:7b168b4b109a0f741078a71b7c4dddaf1d283a5244608f7851f5714fbad273ba
TEST_ARGS :=

##@ Contracts
DUMMY_LOCK_LIB := dummy-lock/target/${RUST_TARGET}/release/libdummy_lock.a
MAIN_LIB := main/target/${RUST_TARGET}/release/libgodwoken_main.a

# deps
contracts: binaries/dummy_lock binaries/main

contracts-via-docker: ${DUMMY_LOCK_LIB} ${MAIN_LIB}
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make contracts"

binaries/dummy_lock: c/entry.c ${DUMMY_LOCK_LIB}
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

binaries/main: c/entry.c c/ckb_syscalls.c ${MAIN_LIB}
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

${DUMMY_LOCK_LIB}: dummy-lock/src/lib.rs
	cd dummy-lock && cargo build --target ${RUST_TARGET} --release

${MAIN_LIB}: main/src/lib.rs
	cd main && cargo build --target ${RUST_TARGET} --release

clean:
	rm binaries/dummy_lock binaries/main
