# contract compile
C_TARGET := riscv64-unknown-elf
CC := $(C_TARGET)-gcc
LD := $(C_TARGET)-gcc
CFLAGS := -O3 -Ideps/molecule -Icontract/deps -Icontract/deps/mmr-c -Icontract/deps/molecule -Icontract/deps/types -Icontract/ckb-c-stdlib -Wall -Werror -Wno-nonnull -Wno-unused-function
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections -Wl,-s
# molecule
MOLC := moleculec
MOLC_VERSION := 0.4.2
GEN_MOL_IN_DIR := types/schemas
# docker pull nervos/ckb-riscv-gnu-toolchain:bionic-20190702
BUILDER_DOCKER := nervos/ckb-riscv-gnu-toolchain@sha256:7b168b4b109a0f741078a71b7c4dddaf1d283a5244608f7851f5714fbad273ba

default: ci

##@ Contracts
C := contract
CONTRACT_HEADERS := $C/common.h $C/registration.c $C/deposit.c
GEN_MOL_OUT_DIR_C := $C/deps/types
GEN_MOL_C_FILES := ${GEN_MOL_OUT_DIR_C}/blockchain.h ${GEN_MOL_OUT_DIR_C}/godwoken.h
${GEN_MOL_OUT_DIR_C}/blockchain.h: ${GEN_MOL_IN_DIR}/blockchain.mol
	${MOLC} --language c --schema-file $< > $@
${GEN_MOL_OUT_DIR_C}/godwoken.h: ${GEN_MOL_IN_DIR}/godwoken.mol
	${MOLC} --language c --schema-file $< > $@
# deps
MMR_C := contract/deps/mmr-c

contracts: $C/binary/dummy_lock $C/binary/main

contracts-via-docker: install-tools ${GEN_MOL_C_FILES}
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make contracts"

contract/binary/dummy_lock: $C/dummy_lock.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

contract/binary/main: $C/main.c ${MMR_C}/mmr.o ${CONTRACT_HEADERS} ${GEN_MOL_C_FILES}
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $< ${MMR_C}/mmr.o

${MMR_C}/mmr.o: ${MMR_C}/mmr.c
	$(CC) $(CFLAGS) $(LDFLAGS) -c -o $@ $<

clean-contracts:
	rm $C/binary/dummy_lock $C/binary/main

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

clean: clean-contracts

# .PHONY:
