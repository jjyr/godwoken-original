default: ci

##@ Generates Schema
.PHONY: gen
GEN_MOL_IN_DIR := types/schemas
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
CONTRACTS := contracts
TESTS := contracts-test

.PHONY: ci
ci: contracts-via-docker check-fmt clippy test bench-test

contracts-via-docker:
	make -C ${CONTRACTS} $@

test: ${GEN_MOL_OUT_DIR}/godwoken.rs
	cd ${TESTS} && cargo test --all --all-features ${TEST_ARGS} -- --nocapture

clippy:
	cd ${TESTS} && cargo clippy --all --all-features --all-targets

check-fmt:
	cd ${TESTS} && cargo fmt --all -- --check

check:
	cd ${TESTS} && cargo check --all --all-targets

fmt:
	cd ${TESTS} && cargo fmt --all

clean:
	make -C ${CONTRACTS} $@

# .PHONY:
