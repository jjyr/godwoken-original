# C compile
C_TARGET := riscv64-unknown-elf
CC := $(C_TARGET)-gcc
LD := $(C_TARGET)-gcc
CFLAGS := -O3 -I c/ckb-c-stdlib -Wall -Werror -Wno-nonnull -Wno-unused-function
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections -Wl,-s

# rust compile
RUST_TARGET := riscv64imac-unknown-none-elf
CARGO_PROJS := main dummy-lock

# CKB binary patcher 
install-binary-patcher:
	which ckb-binary-patcher || \
		cargo install --force --git https://github.com/xxuejie/ckb-binary-patcher.git

# Docker
BUILDER_DOCKER := jjy0/ckb-riscv-rust-toolchain:2019-12-30

build-image:
	docker build -t ${BUILDER_DOCKER} .

##@ Contracts
DUMMY_LOCK_LIB := dummy-lock/target/${RUST_TARGET}/debug/libdummy_lock.a
MAIN_LIB := main/target/${RUST_TARGET}/debug/libgodwoken_main.a
# deps
contracts: binaries/dummy_lock binaries/main

contracts-via-docker:
	docker run -eOWNER=`id -u`:`id -g` --rm -v `pwd`/..:/code ${BUILDER_DOCKER} bash -c "cd /code/contracts && make contracts"

binaries/dummy_lock: c/entry.c ${DUMMY_LOCK_LIB}
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^ && chown $$OWNER $@

binaries/main: c/entry.c c/ckb_syscalls.c ${MAIN_LIB}
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^ && chown $$OWNER $@ && \
		ckb-binary-patcher -i binaries/main -o binaries/main

${DUMMY_LOCK_LIB}:
	cd dummy-lock && cargo build --target ${RUST_TARGET}; \
		chown -R $$OWNER target

${MAIN_LIB}:
	cd main && cargo build --target ${RUST_TARGET}; \
		chown -R $$OWNER target

clean-cargo:
	CURRENT_DIR=`pwd`; \
	for proj in ${CARGO_PROJS} ; do \
        cd $$CURRENT_DIR/$$proj && cargo clean ; \
    done

clean: clean-cargo
	rm binaries/dummy_lock binaries/main || true

.phony: clean ${DUMMY_LOCK_LIB} ${MAIN_LIB}
