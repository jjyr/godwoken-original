# Rust compile
RUST_TARGET := riscv64imac-unknown-none-elf
RUSTFLAGS := "-C link-arg=-s"
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
DUMMY_LOCK := dummy-lock/target/${RUST_TARGET}/release/dummy-lock
MAIN := main/target/${RUST_TARGET}/release/godwoken-main
# deps
contracts: ${MAIN} ${DUMMY_LOCK}
	cp $^ binaries/

contracts-via-docker:
	docker run -eOWNER=`id -u`:`id -g` --rm -v `pwd`/..:/code ${BUILDER_DOCKER} bash -c "cd /code/contracts && make contracts"

${DUMMY_LOCK}:
	cd dummy-lock && RUSTFLAGS=${RUSTFLAGS} cargo build --target ${RUST_TARGET} --release && \
		chown -R $$OWNER target && cd .. && \
		ckb-binary-patcher -i ${DUMMY_LOCK} -o ${DUMMY_LOCK}

${MAIN}:
	cd main && RUSTFLAGS=${RUSTFLAGS} cargo build --target ${RUST_TARGET} --release && \
		chown -R $$OWNER target && cd .. && \
		ckb-binary-patcher -i ${MAIN} -o ${MAIN}

clean-cargo:
	CURRENT_DIR=`pwd`; \
	for proj in ${CARGO_PROJS} ; do \
        cd $$CURRENT_DIR/$$proj && cargo clean ; \
    done

clean: clean-cargo
	rm binaries/dummy_lock binaries/main || true

.PHONY: clean ${DUMMY_LOCK} ${MAIN}
