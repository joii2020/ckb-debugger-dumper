TARGET := riscv64-unknown-linux-gnu
CC := $(TARGET)-gcc
LD := $(TARGET)-gcc
OBJCOPY := $(TARGET)-objcopy

CLANG := clang
LLVM_OBJCOPY := llvm-objcopy

INCLUDE_FLAGS := \
	-I c/deps/sparse-merkle-tree/src \
	-I c/deps/sparse-merkle-tree/c \
	-I c/deps/sparse-merkle-tree \
	-I c/deps/ckb-c-lib \
	-I c/deps/ckb-c-lib/libc \
	-I c/deps/ckb-c-lib/molecule \
	-I c \
	-I build

WARNING_FLAGS := \
	-Wall -Werror \
	-Wno-nonnull \
	-Wno-nonnull-compare \
	-Wno-unused-function

WARNING_FLAGS_CLANG := \
	-Wall -Werror

CFLAGS := \
	-fPIC -O3 -fno-builtin-printf -fno-builtin-memcmp -nostdinc -nostdlib -nostartfiles -fvisibility=hidden -fdata-sections -ffunction-sections -g \
	${WARNING_FLAGS} \
	${INCLUDE_FLAGS}

CFLAGS_CLANG := \
	--target=riscv64 -march=rv64imc_zba_zbb_zbc_zbs \
	-fPIC -O3 -fno-builtin-printf -fno-builtin-memcmp -nostdinc -nostdlib -fvisibility=hidden -fdata-sections -ffunction-sections -g \
	${WARNING_FLAGS_CLANG} \
	${INCLUDE_FLAGS}

LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections

BUILDER_DOCKER := \
	nervos/ckb-riscv-gnu-toolchain@sha256:d3f649ef8079395eb25a21ceaeb15674f47eaa2d8cc23adc8bcdae3d5abce6ec

all: init build/dump build/always_failed build/always_success

all-via-docker: ${PROTOCOL_HEADER}
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make"

all-clang: init
	make CC=$(CLANG) OBJCOPY=$(LLVM_OBJCOPY) CFLAGS="$(CFLAGS_CLANG)" build/dump build/always_failed build/always_success

init:
	mkdir -p build

build/dump: c/dump.c c/output.h
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<
	$(OBJCOPY) --only-keep-debug $@ $@.debug
	$(OBJCOPY) --strip-debug --strip-all $@

build/always_failed: c/always_failed.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<
	$(OBJCOPY) --only-keep-debug $@ $@.debug
	$(OBJCOPY) --strip-debug --strip-all $@

build/always_success: c/always_success.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<
	$(OBJCOPY) --only-keep-debug $@ $@.debug
	$(OBJCOPY) --strip-debug --strip-all $@

build/ckb-debugger-bins: build_ckb-debugger.sh
	bash ./build_ckb-debugger.sh

clean:
	rm -rf build
