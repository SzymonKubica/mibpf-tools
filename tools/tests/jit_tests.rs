mod common;

use mibpf_tools::load_env;
use common::test_jit_execution;
use mibpf_common::BinaryFileLayout;

/// Tests for the simple programs to ensure that the jit compiler works correctly.

#[tokio::test]
async fn jit_add() {
    test_jit("jit_basic-add.c").await;
}

#[tokio::test]
async fn jit_subtract() {
    test_jit("jit_basic-subtract.c").await;
}

#[tokio::test]
async fn jit_multiply() {
    test_jit("jit_basic-multiply.c").await;
}

#[tokio::test]
async fn jit_add_reg() {
    test_jit("jit_add-reg.c").await;
}

#[tokio::test]
async fn jit_multiply_reg() {
    test_jit("jit_multiply-reg.c").await;
}

#[tokio::test]
async fn jit_subtract_reg() {
    test_jit("jit_subtract-reg.c").await;
}

#[ignore]
#[tokio::test]
async fn jit_fletcher() {
    test_jit("jit_fletcher32_checksum.c").await;
}


async fn test_jit(test_program: &str) {
    let env = load_env();
    test_jit_execution(test_program, BinaryFileLayout::OnlyTextSection, &env).await;
}
