// This is free and unencumbered software released into the public domain.

use ethnum::I256;
use std::ops::{Not, Shl, Shr};
use ux::*;

use crate::state::{Machine, Memory, Stack, Storage, Word, MAX_STACK_DEPTH, ONE, ZERO};

pub(crate) static mut EVM: Machine = Machine {
    gas_limit: 10_000_000,
    gas_used: 0,
    gas_price: 0, // gas is ultimately paid in $NEAR
    stack: Stack {
        depth: 0,
        slots: [ZERO; MAX_STACK_DEPTH],
    },
    memory: Memory { bytes: Vec::new() },
    storage: Storage { entries: None },
    call_value: Word::ZERO,
    call_data: Vec::new(),
    code: Vec::new(),
};

#[no_mangle]
pub unsafe fn _init_evm(_table_offset: u32) {
    #[cfg(target_os = "wasi")]
    {
        let mut args = std::env::args();
        let _ = args.next(); // consume the program name
        EVM.call_data = match args.next() {
            None => Vec::new(),
            Some(hexbytes) => match hex::decode(hexbytes) {
                Err(err) => panic!("{}", err),
                Ok(bytes) => bytes,
            },
        };
        EVM.call_value = match args.next() {
            None => ZERO,
            Some(s) => Word::from(s.parse::<u32>().unwrap_or(0)),
        };
        //eprintln!("EVM.call_data={:?} EVM.call_value={:?}", EVM.call_data, EVM.call_value);
    }
}

#[no_mangle]
pub unsafe fn _pop_u32() -> u32 {
    EVM.stack.pop().as_u32()
}

#[no_mangle]
pub unsafe fn stop() {
    EVM.gas_used += 0;
    EVM.stack.clear();
    #[cfg(target_os = "wasi")]
    {
        eprintln!("STOP");
        std::process::exit(0) // EX_OK
    }
    #[cfg(not(target_os = "wasi"))]
    todo!("STOP") // TODO
}

#[no_mangle]
pub unsafe fn add() {
    EVM.gas_used += 3;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(a + b);
}

#[no_mangle]
pub unsafe fn mul() {
    EVM.gas_used += 5;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(a * b);
}

#[no_mangle]
pub unsafe fn sub() {
    EVM.gas_used += 3;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(a - b);
}

#[no_mangle]
pub unsafe fn div() {
    EVM.gas_used += 5;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(if b == ZERO { ZERO } else { a / b });
}

#[no_mangle]
pub unsafe fn sdiv() {
    EVM.gas_used += 5;
    let a = EVM.stack.pop().as_i256();
    let b = EVM.stack.pop().as_i256();
    EVM.stack.push(if b == I256::ZERO {
        ZERO
    } else {
        (a / b).as_u256()
    });
}

#[no_mangle]
pub unsafe fn r#mod() {
    EVM.gas_used += 5;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(if b == ZERO { ZERO } else { a % b });
}

#[no_mangle]
pub unsafe fn smod() {
    EVM.gas_used += 5;
    let a = EVM.stack.pop().as_i256();
    let b = EVM.stack.pop().as_i256();
    EVM.stack.push(if b == I256::ZERO {
        ZERO
    } else {
        (a % b).as_u256()
    });
}

#[no_mangle]
pub unsafe fn addmod() {
    EVM.gas_used += 8;
    // TODO: need to use 512-bit arithmetic here to prevent overflow before taking the modulus
    let (a, b, n) = EVM.stack.pop3();
    let result = if n == ZERO { ZERO } else { (a + b) % n };
    EVM.stack.push(result);
}

#[no_mangle]
pub unsafe fn mulmod() {
    EVM.gas_used += 8;
    // TODO: need to use 512-bit arithmetic here to prevent overflow before taking the modulus
    let (a, b, n) = EVM.stack.pop3();
    let result = if n == ZERO { ZERO } else { (a * b) % n };
    EVM.stack.push(result);
}

#[no_mangle]
pub unsafe fn exp() {
    EVM.gas_used += 10;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(a.pow(b.try_into().unwrap()));
}

#[no_mangle]
pub unsafe fn signextend() {
    EVM.gas_used += 5;
    let (op1, op2) = EVM.stack.pop2();
    let result = if op1 < ethnum::U256::new(32) {
        // `as_u32` works since op1 < 32
        let bit_index = (8 * op1.as_u32() + 7) as usize;
        let word = if bit_index < 128 {
            op2.low()
        } else {
            op2.high()
        };
        let bit = word & (1 << (bit_index % 128)) != 0;
        let mask = (ONE << bit_index) - ONE;
        if bit {
            op2 | !mask
        } else {
            op2 & mask
        }
    } else {
        op2
    };
    EVM.stack.push(result);
}

#[no_mangle]
pub unsafe fn lt() {
    EVM.gas_used += 3;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(if a < b { ONE } else { ZERO });
}

#[no_mangle]
pub unsafe fn gt() {
    EVM.gas_used += 3;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(if a > b { ONE } else { ZERO });
}

#[no_mangle]
pub unsafe fn slt() {
    EVM.gas_used += 3;
    let a = EVM.stack.pop().as_i256();
    let b = EVM.stack.pop().as_i256();
    EVM.stack.push(if a < b { ONE } else { ZERO });
}

#[no_mangle]
pub unsafe fn sgt() {
    EVM.gas_used += 3;
    let a = EVM.stack.pop().as_i256();
    let b = EVM.stack.pop().as_i256();
    EVM.stack.push(if a > b { ONE } else { ZERO });
}

#[no_mangle]
pub unsafe fn eq() {
    EVM.gas_used += 3;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(if a == b { ONE } else { ZERO });
}

#[no_mangle]
pub unsafe fn iszero() {
    EVM.gas_used += 3;
    let a = EVM.stack.pop();
    EVM.stack.push(if a == ZERO { ONE } else { ZERO });
}

#[no_mangle]
pub unsafe fn and() {
    EVM.gas_used += 3;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(a & b);
}

#[no_mangle]
pub unsafe fn or() {
    EVM.gas_used += 3;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(a | b);
}

#[no_mangle]
pub unsafe fn xor() {
    EVM.gas_used += 3;
    let (a, b) = EVM.stack.pop2();
    EVM.stack.push(a ^ b);
}

#[no_mangle]
pub unsafe fn not() {
    EVM.gas_used += 3;
    let a = EVM.stack.pop();
    EVM.stack.push(a.not());
}

#[no_mangle]
pub unsafe fn byte() {
    EVM.gas_used += 3;
    todo!("BYTE") // TODO
}

#[no_mangle]
pub unsafe fn shl() {
    EVM.gas_used += 3;
    let (shift, value) = EVM.stack.pop2();
    let result = if value == ZERO || shift > Word::from(255u8) {
        ZERO
    } else {
        value.shl(shift)
    };
    EVM.stack.push(result);
}

#[no_mangle]
pub unsafe fn shr() {
    EVM.gas_used += 3;
    let (shift, value) = EVM.stack.pop2();
    let result = if value == ZERO || shift > Word::from(255u8) {
        ZERO
    } else {
        value.shr(shift)
    };
    EVM.stack.push(result);
}

#[no_mangle]
pub unsafe fn sar() {
    EVM.gas_used += 3;
    todo!("SAR") // TODO
}

#[no_mangle]
pub unsafe fn sha3() {
    EVM.gas_used += 30;
    todo!("SHA3") // TODO
}

#[no_mangle]
pub unsafe fn address() {
    EVM.gas_used += 2;
    todo!("ADDRESS") // TODO: NEAR SDK, --address
}

#[no_mangle]
pub unsafe fn balance() {
    EVM.gas_used += 100;
    todo!("BALANCE") // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn origin() {
    EVM.gas_used += 2;
    todo!("ORIGIN") // TODO: NEAR SDK, --origin
}

#[no_mangle]
pub unsafe fn caller() {
    EVM.gas_used += 2;
    todo!("CALLER") // TODO: NEAR SDK, --caller
}

#[no_mangle]
pub unsafe fn callvalue() {
    EVM.gas_used += 2;
    EVM.stack.push(EVM.call_value);
}

#[no_mangle]
pub unsafe fn calldataload() {
    EVM.gas_used += 3;
    todo!("CALLDATALOAD") // TODO!
}

#[no_mangle]
pub unsafe fn calldatasize() {
    EVM.gas_used += 2;
    EVM.stack.push(Word::from(EVM.call_data.len() as u32));
}

#[no_mangle]
pub unsafe fn calldatacopy() {
    EVM.gas_used += 3;
    todo!("CALLDATACOPY") // TODO!
}

#[no_mangle]
pub unsafe fn codesize() {
    EVM.gas_used += 2;
    EVM.stack.push(Word::from(EVM.code.len() as u32));
}

#[no_mangle]
pub unsafe fn codecopy() {
    EVM.gas_used += 3;
    todo!("CODECOPY") // TODO!
}

#[no_mangle]
pub unsafe fn gasprice() {
    EVM.gas_used += 2;
    EVM.stack.push(Word::from(EVM.gas_price));
}

#[no_mangle]
pub unsafe fn extcodesize() {
    EVM.gas_used += 100;
    todo!("EXTCODESIZE") // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn extcodecopy() {
    EVM.gas_used += 100;
    todo!("EXTCODECOPY") // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn returndatasize() {
    EVM.gas_used += 2;
    todo!("RETURNDATASIZE") // TODO
}

#[no_mangle]
pub unsafe fn returndatacopy() {
    EVM.gas_used += 3;
    todo!("RETURNDATACOPY") // TODO
}

#[no_mangle]
pub unsafe fn extcodehash() {
    EVM.gas_used += 100;
    todo!("EXTCODEHASH") // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn blockhash() {
    EVM.gas_used += 20;
    EVM.stack.push(ZERO) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn coinbase() {
    EVM.gas_used += 2;
    EVM.stack.push(ZERO) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn timestamp() {
    EVM.gas_used += 2;
    EVM.stack.push(ZERO) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn number() {
    EVM.gas_used += 2;
    EVM.stack.push(ZERO) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn difficulty() {
    EVM.gas_used += 2;
    EVM.stack.push(ZERO)
}

#[no_mangle]
pub unsafe fn gaslimit() {
    EVM.gas_used += 2;
    EVM.stack.push(Word::from(EVM.gas_limit))
}

#[no_mangle]
pub unsafe fn chainid() {
    EVM.gas_used += 2;
    EVM.stack.push(ZERO) // TODO: --chain-id=mainnet
}

#[no_mangle]
pub unsafe fn selfbalance() {
    EVM.gas_used += 5;
    EVM.stack.push(ZERO) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn basefee() {
    EVM.gas_used += 2;
    EVM.stack.push(ZERO)
}

#[no_mangle]
pub unsafe fn pop() {
    EVM.gas_used += 2;
    let _ = EVM.stack.pop();
}

#[no_mangle]
pub unsafe fn mload() {
    EVM.gas_used += 3;
    let offset = EVM.stack.pop();
    let value = EVM.memory.load_word(offset.try_into().unwrap());
    EVM.stack.push(value);
}

#[no_mangle]
pub unsafe fn mstore() {
    EVM.gas_used += 3;
    let (offset, value) = EVM.stack.pop2();
    EVM.memory.store_word(offset.try_into().unwrap(), value);
}

#[no_mangle]
pub unsafe fn mstore8() {
    EVM.gas_used += 3;
    let (offset, value) = (EVM.stack.pop(), EVM.stack.pop() & 0xFF);
    EVM.memory
        .store_byte(offset.try_into().unwrap(), value.try_into().unwrap());
}

#[no_mangle]
pub unsafe fn sload() {
    EVM.gas_used += 100;
    let key = EVM.stack.pop();
    let value = EVM.storage.load_word(key);
    EVM.stack.push(value);
}

#[no_mangle]
pub unsafe fn sstore() {
    EVM.gas_used += 100;
    let (key, value) = EVM.stack.pop2();
    EVM.storage.store_word(key, value);
}

#[no_mangle]
pub unsafe fn jump() {
    EVM.gas_used += 8;
    todo!("JUMP") // TODO
}

#[no_mangle]
pub unsafe fn jumpi() -> u32 {
    EVM.gas_used += 10;
    //let pc = EVM.stack.pop(); // never pushed on the stack for static jumps
    let cond = EVM.stack.pop();
    if cond != Word::ZERO {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe fn pc() {
    EVM.gas_used += 2;
    EVM.stack.push(ZERO) // TODO: --fno-program-counter
}

#[no_mangle]
pub unsafe fn msize() {
    EVM.gas_used += 2;
    EVM.stack.push(Word::from(EVM.memory.size() as u64));
}

#[no_mangle]
pub unsafe fn gas() {
    EVM.gas_used += 2;
    EVM.stack.push(Word::from(EVM.gas_limit - EVM.gas_used)) // TODO: --fno-gas-accounting
}

#[no_mangle]
pub unsafe fn jumpdest() {
    unreachable!("JUMPDEST")
}

#[no_mangle]
pub unsafe fn push1(word: u8) {
    EVM.gas_used += 3;
    EVM.stack.push(Word::from(word));
}

#[no_mangle]
pub unsafe fn push2(word: u16) {
    push4(word.into())
}

#[no_mangle]
pub unsafe fn push3(word: u24) {
    push4(word.into())
}

#[no_mangle]
pub unsafe fn push4(word: u32) {
    EVM.gas_used += 3;
    EVM.stack.push(Word::from(word));
}

#[no_mangle]
pub unsafe fn push5(word: u40) {
    push8(word.into())
}

#[no_mangle]
pub unsafe fn push6(word: u48) {
    push8(word.into())
}

#[no_mangle]
pub unsafe fn push7(word: u56) {
    push8(word.into())
}

#[no_mangle]
pub unsafe fn push8(word: u64) {
    EVM.gas_used += 3;
    EVM.stack.push(Word::from(word));
}

#[no_mangle]
pub unsafe fn push9(word: /*u72*/ u128) {
    push16(word)
}

#[no_mangle]
pub unsafe fn push10(word: /*u80*/ u128) {
    push16(word)
}

#[no_mangle]
pub unsafe fn push11(word: /*u88*/ u128) {
    push16(word)
}

#[no_mangle]
pub unsafe fn push12(word: /*u96*/ u128) {
    push16(word)
}

#[no_mangle]
pub unsafe fn push13(word: /*u104*/ u128) {
    push16(word)
}

#[no_mangle]
pub unsafe fn push14(word: /*u112*/ u128) {
    push16(word)
}

#[no_mangle]
pub unsafe fn push15(word: /*u120*/ u128) {
    push16(word)
}

#[no_mangle]
pub unsafe fn push16(word: u128) {
    EVM.gas_used += 3;
    EVM.stack.push(Word::from_words(0, word));
}

#[no_mangle]
pub unsafe fn push17(word_0: u64, word_1: u64, word_2: u8) {
    push24(word_0, word_1, word_2.into())
}

#[no_mangle]
pub unsafe fn push18(word_0: u64, word_1: u64, word_2: u16) {
    push24(word_0, word_1, word_2.into())
}

#[no_mangle]
pub unsafe fn push19(word_0: u64, word_1: u64, word_2: u24) {
    push24(word_0, word_1, word_2.into())
}

#[no_mangle]
pub unsafe fn push20(word_0: u64, word_1: u64, word_2: u32) {
    push24(word_0, word_1, word_2.into())
}

#[no_mangle]
pub unsafe fn push21(word_0: u64, word_1: u64, word_2: u40) {
    push24(word_0, word_1, word_2.into())
}

#[no_mangle]
pub unsafe fn push22(word_0: u64, word_1: u64, word_2: u48) {
    push24(word_0, word_1, word_2.into())
}

#[no_mangle]
pub unsafe fn push23(word_0: u64, word_1: u64, word_2: u56) {
    push24(word_0, word_1, word_2.into())
}

#[no_mangle]
pub unsafe fn push24(word_0: u64, word_1: u64, word_2: u64) {
    push32(word_0, word_1, word_2, 0);
}

#[no_mangle]
pub unsafe fn push25(word_0: u64, word_1: u64, word_2: u64, word_3: u8) {
    push32(word_0, word_1, word_2, word_3.into())
}

#[no_mangle]
pub unsafe fn push26(word_0: u64, word_1: u64, word_2: u64, word_3: u16) {
    push32(word_0, word_1, word_2, word_3.into())
}

#[no_mangle]
pub unsafe fn push27(word_0: u64, word_1: u64, word_2: u64, word_3: u24) {
    push32(word_0, word_1, word_2, word_3.into())
}

#[no_mangle]
pub unsafe fn push28(word_0: u64, word_1: u64, word_2: u64, word_3: u32) {
    push32(word_0, word_1, word_2, word_3.into())
}

#[no_mangle]
pub unsafe fn push29(word_0: u64, word_1: u64, word_2: u64, word_3: u40) {
    push32(word_0, word_1, word_2, word_3.into())
}

#[no_mangle]
pub unsafe fn push30(word_0: u64, word_1: u64, word_2: u64, word_3: u48) {
    push32(word_0, word_1, word_2, word_3.into())
}

#[no_mangle]
pub unsafe fn push31(word_0: u64, word_1: u64, word_2: u64, word_3: u56) {
    push32(word_0, word_1, word_2, word_3.into())
}

#[no_mangle]
pub unsafe fn push32(word_0: u64, word_1: u64, word_2: u64, word_3: u64) {
    EVM.gas_used += 3;
    let mut bytes: [u8; 32] = [0; 32];
    bytes[0..8].copy_from_slice(&word_0.to_le_bytes());
    bytes[8..16].copy_from_slice(&word_1.to_le_bytes());
    bytes[16..24].copy_from_slice(&word_2.to_le_bytes());
    bytes[24..32].copy_from_slice(&word_3.to_le_bytes());
    EVM.stack.push(Word::from_le_bytes(bytes));
}

#[no_mangle]
pub unsafe fn dup1() {
    EVM.gas_used += 3;
    EVM.stack.push(EVM.stack.peek());
}

#[no_mangle]
pub unsafe fn dup2() {
    dup(2)
}

#[no_mangle]
pub unsafe fn dup3() {
    dup(3)
}

#[no_mangle]
pub unsafe fn dup4() {
    dup(4)
}

#[no_mangle]
pub unsafe fn dup5() {
    dup(5)
}

#[no_mangle]
pub unsafe fn dup6() {
    dup(6)
}

#[no_mangle]
pub unsafe fn dup7() {
    dup(7)
}

#[no_mangle]
pub unsafe fn dup8() {
    dup(8)
}

#[no_mangle]
pub unsafe fn dup9() {
    dup(9)
}

#[no_mangle]
pub unsafe fn dup10() {
    dup(10)
}

#[no_mangle]
pub unsafe fn dup11() {
    dup(11)
}

#[no_mangle]
pub unsafe fn dup12() {
    dup(12)
}

#[no_mangle]
pub unsafe fn dup13() {
    dup(13)
}

#[no_mangle]
pub unsafe fn dup14() {
    dup(14)
}

#[no_mangle]
pub unsafe fn dup15() {
    dup(15)
}

#[no_mangle]
pub unsafe fn dup16() {
    dup(16)
}

unsafe fn dup(n: u8) {
    assert!((1..=16).contains(&n));
    EVM.gas_used += 3;
    EVM.stack.push(EVM.stack.peek_n(n as usize - 1));
}

#[no_mangle]
pub unsafe fn swap1() {
    swap(1)
}

#[no_mangle]
pub unsafe fn swap2() {
    swap(2)
}

#[no_mangle]
pub unsafe fn swap3() {
    swap(3)
}

#[no_mangle]
pub unsafe fn swap4() {
    swap(4)
}

#[no_mangle]
pub unsafe fn swap5() {
    swap(5)
}

#[no_mangle]
pub unsafe fn swap6() {
    swap(6)
}

#[no_mangle]
pub unsafe fn swap7() {
    swap(7)
}

#[no_mangle]
pub unsafe fn swap8() {
    swap(8)
}

#[no_mangle]
pub unsafe fn swap9() {
    swap(9)
}

#[no_mangle]
pub unsafe fn swap10() {
    swap(10)
}

#[no_mangle]
pub unsafe fn swap11() {
    swap(11)
}

#[no_mangle]
pub unsafe fn swap12() {
    swap(12)
}

#[no_mangle]
pub unsafe fn swap13() {
    swap(13)
}

#[no_mangle]
pub unsafe fn swap14() {
    swap(14)
}

#[no_mangle]
pub unsafe fn swap15() {
    swap(15)
}

#[no_mangle]
pub unsafe fn swap16() {
    swap(16)
}

unsafe fn swap(n: u8) {
    assert!((1..=16).contains(&n));
    EVM.gas_used += 3;
    EVM.stack.swap(n.into())
}

#[no_mangle]
pub unsafe fn log0() {
    EVM.gas_used += 375;
    let (offset, size) = EVM.stack.pop2();
    let data = EVM.memory.slice(offset.as_usize(), size.as_usize());
    #[cfg(target_os = "wasi")]
    eprintln!("LOG0 0x{}", hex::encode(data));
    #[cfg(not(target_os = "wasi"))]
    todo!("LOG0 0x{}", hex::encode(data)) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn log1() {
    EVM.gas_used += 750;
    let (offset, size) = EVM.stack.pop2();
    let topic = EVM.stack.pop();
    let data = EVM.memory.slice(offset.as_usize(), size.as_usize());
    #[cfg(target_os = "wasi")]
    eprintln!(
        "LOG1 0x{} 0x{}",
        hex::encode(data),
        hex::encode(topic.to_be_bytes())
    );
    #[cfg(not(target_os = "wasi"))]
    todo!(
        "LOG1 0x{} 0x{}",
        hex::encode(data),
        hex::encode(topic.to_be_bytes())
    ) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn log2() {
    EVM.gas_used += 1125;
    let (offset, size) = EVM.stack.pop2();
    let (topic1, topic2) = EVM.stack.pop2();
    let data = EVM.memory.slice(offset.as_usize(), size.as_usize());
    #[cfg(target_os = "wasi")]
    eprintln!(
        "LOG2 0x{} 0x{} 0x{}",
        hex::encode(data),
        hex::encode(topic1.to_be_bytes()),
        hex::encode(topic2.to_be_bytes())
    );
    #[cfg(not(target_os = "wasi"))]
    todo!(
        "LOG2 0x{} 0x{} 0x{}",
        hex::encode(data),
        hex::encode(topic1.to_be_bytes()),
        hex::encode(topic2.to_be_bytes())
    ) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn log3() {
    EVM.gas_used += 1500;
    let (offset, size) = EVM.stack.pop2();
    let (topic1, topic2, topic3) = EVM.stack.pop3();
    let data = EVM.memory.slice(offset.as_usize(), size.as_usize());
    #[cfg(target_os = "wasi")]
    eprintln!(
        "LOG3 0x{} 0x{} 0x{} 0x{}",
        hex::encode(data),
        hex::encode(topic1.to_be_bytes()),
        hex::encode(topic2.to_be_bytes()),
        hex::encode(topic3.to_be_bytes())
    );
    #[cfg(not(target_os = "wasi"))]
    todo!(
        "LOG3 0x{} 0x{} 0x{} 0x{}",
        hex::encode(data),
        hex::encode(topic1.to_be_bytes()),
        hex::encode(topic2.to_be_bytes()),
        hex::encode(topic3.to_be_bytes())
    ) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn log4() {
    EVM.gas_used += 1875;
    let (offset, size) = EVM.stack.pop2();
    let (topic1, topic2, topic3, topic4) = EVM.stack.pop4();
    let data = EVM.memory.slice(offset.as_usize(), size.as_usize());
    #[cfg(target_os = "wasi")]
    eprintln!(
        "LOG4 0x{} 0x{} 0x{} 0x{} 0x{}",
        hex::encode(data),
        hex::encode(topic1.to_be_bytes()),
        hex::encode(topic2.to_be_bytes()),
        hex::encode(topic3.to_be_bytes()),
        hex::encode(topic4.to_be_bytes())
    );
    #[cfg(not(target_os = "wasi"))]
    todo!(
        "LOG4 0x{} 0x{} 0x{} 0x{} 0x{}",
        hex::encode(data),
        hex::encode(topic1.to_be_bytes()),
        hex::encode(topic2.to_be_bytes()),
        hex::encode(topic3.to_be_bytes()),
        hex::encode(topic4.to_be_bytes())
    ) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn create() {
    EVM.gas_used += 32000;
    todo!("CREATE") // TODO
}

#[no_mangle]
pub unsafe fn call() {
    EVM.gas_used += 100;
    todo!("CALL") // TODO
}

#[no_mangle]
pub unsafe fn callcode() {
    EVM.gas_used += 100;
    todo!("CALLCODE") // TODO
}

#[no_mangle]
pub unsafe fn r#return() {
    EVM.gas_used += 0;
    let (offset, size) = EVM.stack.pop2();
    let data = EVM.memory.slice(offset.as_usize(), size.as_usize());
    #[cfg(target_os = "wasi")]
    {
        eprintln!("RETURN 0x{}", hex::encode(data));
        std::process::exit(0); // EX_OK
    }
    #[cfg(not(target_os = "wasi"))]
    todo!("RETURN 0x{}", hex::encode(data)) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn delegatecall() {
    EVM.gas_used += 100;
    todo!("DELEGATECALL") // TODO
}

#[no_mangle]
pub unsafe fn create2() {
    EVM.gas_used += 32000;
    todo!("CREATE2") // TODO
}

#[no_mangle]
pub unsafe fn staticcall() {
    EVM.gas_used += 100;
    todo!("STATICCALL") // TODO
}

#[no_mangle]
pub unsafe fn revert() {
    EVM.gas_used += 0;
    let (offset, size) = EVM.stack.pop2();
    let data = EVM.memory.slice(offset.as_usize(), size.as_usize());
    #[cfg(target_os = "wasi")]
    {
        eprintln!("REVERT 0x{}", hex::encode(data));
        std::process::exit(64); // EX_USAGE
    }
    #[cfg(not(target_os = "wasi"))]
    todo!("REVERT 0x{}", hex::encode(data)) // TODO: NEAR SDK
}

#[no_mangle]
pub unsafe fn invalid() {
    EVM.gas_used += 0;
    #[cfg(target_os = "wasi")]
    {
        eprintln!("INVALID");
        std::process::exit(70) // EX_SOFTWARE
    }
    #[cfg(not(target_os = "wasi"))]
    todo!("INVALID") // TODO
}

#[no_mangle]
pub unsafe fn selfdestruct() {
    EVM.gas_used += 5000;
    todo!("SELFDESTRUCT") // TODO: state reset
}
