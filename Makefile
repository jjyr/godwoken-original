# contract compile
C_TARGET := riscv64-unknown-elf
CC := $(C_TARGET)-gcc
LD := $(C_TARGET)-gcc
CFLAGS := -O3 -Ideps/molecule -I contract/deps -I contract/deps/molecule -I contract/deps/types -I contract/ckb-c-stdlib -Wall -Werror -Wno-nonnull-compare -Wno-unused-function
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections -Wl,-s
# molecule
MOLC := moleculec
MOLC_VERSION := 0.4.2
GEN_MOL_IN_DIR := types/schemas
# docker pull nervos/ckb-riscv-gnu-toolchain:bionic-20190702
BUILDER_DOCKER := nervos/ckb-riscv-gnu-toolchain@sha256:7b168b4b109a0f741078a71b7c4dddaf1d283a5244608f7851f5714fbad273ba

default: ci

##@ Contracts
GEN_MOL_OUT_DIR_C := contract/deps/types
GEN_MOL_C_FILES := ${GEN_MOL_OUT_DIR_C}/blockchain.h ${GEN_MOL_OUT_DIR_C}/godwoken.h
${GEN_MOL_OUT_DIR_C}/blockchain.h: ${GEN_MOL_IN_DIR}/blockchain.mol
	${MOLC} --language c --schema-file $< > $@
${GEN_MOL_OUT_DIR_C}/godwoken.h: ${GEN_MOL_IN_DIR}/godwoken.mol
	${MOLC} --language c --schema-file $< > $@

contracts: contract/binary/dummy_lock contract/binary/main

contracts-via-docker: install-tools ${GEN_MOL_C_FILES}
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make contracts"

contract/binary/dummy_lock: contract/dummy_lock.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

contract/binary/main: contract/main.c ${GEN_MOL_C_FILES}
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

##@ Generates Schema
.PHONY: gen
GEN_MOL_OUT_DIR := types/src/generated
GEN_MOL_FILES := ${GEN_MOL_OUT_DIR}/godwoken.rs
gen: check-moleculec-version ${GEN_MOL_FILES} # Generate Files

.PHONY: check-moleculec-version
check-moleculec-version:
	test "$$(${MOLC} --version | awk '{ print $$2 }' | tr -d ' ')" = ${MOLC_VERSION}

${GEN_MOL_OUT_DIR}/godwoken.rs: ${GEN_MOL_IN_DIR}/godwoken.mol
	${MOLC} --language rust --schema-file $< | rustfmt > $@

install-tools:
	test "$$(${MOLC} --version)" == "Moleculec ${MOLC_VERSION}" || \
		cargo install --force --version "${MOLC_VERSION}" "${MOLC}"

##@ Development
.PHONY: ci
ci: contracts-via-docker fmt clippy test bench-test

test:
	cargo test --all --all-features

bench-test:
	cargo bench -- --test

clippy:
	cargo clippy --all --all-features --all-targets

fmt:
	cargo fmt --all -- --check

check:
	cargo check --all --all-targets

# .PHONY:
