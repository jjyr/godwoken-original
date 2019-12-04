# C compile
C_TARGET := riscv64-unknown-elf
CC := $(C_TARGET)-gcc
LD := $(C_TARGET)-gcc
CFLAGS := -O3 -Wall -Werror -Wno-nonnull -Wno-unused-function
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections -Wl,-s

# rust compile
RUST_TARGET := riscv64imac-unknown-none-elf
# molecule
MOLC := moleculec
MOLC_VERSION := 0.4.2
GEN_MOL_IN_DIR := types/schemas
# docker pull nervos/ckb-riscv-gnu-toolchain:bionic-20190702
BUILDER_DOCKER := nervos/ckb-riscv-gnu-toolchain@sha256:7b168b4b109a0f741078a71b7c4dddaf1d283a5244608f7851f5714fbad273ba
TEST_ARGS :=

##@ Contracts
DUMMY_LOCK_LIB := dummy-lock/target/${RUST_TARGET}/release/libdummy_lock.a
# deps
contracts: binaries/dummy_lock

contracts-via-docker: ${DUMMY_LOCK_LIB}
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make contracts"

binaries/dummy_lock: c/entry.c ${DUMMY_LOCK_LIB}
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

${DUMMY_LOCK_LIB}: dummy-lock/src/lib.rs
	cd dummy-lock && cargo build --target ${RUST_TARGET} --release

clean:
	rm binaries/dummy_lock
