// This is free and unencumbered software released into the public domain.

use evm_rs::Opcode;
use wasm_encoder::Instruction;

pub fn encode_push(op: &Opcode) -> Vec<Instruction> {
    let mut result = vec![];
    use Opcode::*;
    match op {
        PUSH1(b) => {
            result.push(Instruction::I32Const(*b as i32));
        }
        PUSHn(n, v, _) if *n <= 4 => {
            result.push(Instruction::I32Const(v.as_i32()));
        }
        PUSHn(n, v, _) if *n <= 8 => {
            result.push(Instruction::I64Const(v.as_i64()));
        }
        PUSHn(n, _, bs) if *n <= 16 => {
            assert!(bs.len() > 8 && bs.len() <= 16);
            assert_eq!(bs.len(), *n as usize);
            let mut buffer: [u8; 16] = [0; 16];
            buffer[16-bs.len()..].copy_from_slice(bs);
            let word_1 = i64::from_be_bytes(buffer[0..8].try_into().unwrap());
            let word_0 = i64::from_be_bytes(buffer[8..16].try_into().unwrap());
            result.push(Instruction::I64Const(word_0));
            result.push(Instruction::I64Const(word_1));
        }
        PUSHn(n, _, bs) if *n <= 24 => {
            assert!(bs.len() > 16 && bs.len() <= 24);
            assert_eq!(bs.len(), *n as usize);
            let mut buffer: [u8; 24] = [0; 24];
            buffer[24-bs.len()..].copy_from_slice(bs);
            let word_2 = i64::from_be_bytes(buffer[0..8].try_into().unwrap());
            let word_1 = i64::from_be_bytes(buffer[8..16].try_into().unwrap());
            let word_0 = i64::from_be_bytes(buffer[16..24].try_into().unwrap());
            result.push(Instruction::I64Const(word_0));
            result.push(Instruction::I64Const(word_1));
            result.push(Instruction::I64Const(word_2));
        }
        PUSHn(n, _, bs) /*if *n <= 32*/ => {
            assert!(bs.len() > 24 && bs.len() <= 32);
            assert_eq!(bs.len(), *n as usize);
            let mut buffer: [u8; 32] = [0; 32];
            buffer[32-bs.len()..].copy_from_slice(bs);
            let word_3 = i64::from_be_bytes(buffer[0..8].try_into().unwrap());
            let word_2 = i64::from_be_bytes(buffer[8..16].try_into().unwrap());
            let word_1 = i64::from_be_bytes(buffer[16..24].try_into().unwrap());
            let word_0 = i64::from_be_bytes(buffer[24..32].try_into().unwrap());
            result.push(Instruction::I64Const(word_0));
            result.push(Instruction::I64Const(word_1));
            result.push(Instruction::I64Const(word_2));
            result.push(Instruction::I64Const(word_3));
        }
        _ => unreachable!("should not be called for instructions different from push")
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethnum::u256;

    #[test]
    fn push1() {
        let op = Opcode::PUSH1(0x01);
        let insns = encode_push(&op);
        assert_eq!(insns.len(), 1);
        match insns[0] {
            Instruction::I32Const(v) => assert_eq!(v, 0x01),
            _ => panic!("not an I32Const"),
        }
    }

    #[test]
    fn push2() {
        let op = Opcode::PUSHn(2, u256::from(0x0123_u16), vec![0x01, 0x23]);
        let insns = encode_push(&op);
        assert_eq!(insns.len(), 1);
        match insns[0] {
            Instruction::I32Const(v) => assert_eq!(v, 0x0123),
            _ => panic!("not an I32Const"),
        }
    }

    #[test]
    fn push4() {
        let op = Opcode::PUSHn(4, u256::from(0x01234567_u32), vec![0x01, 0x23, 0x45, 0x67]);
        let insns = encode_push(&op);
        assert_eq!(insns.len(), 1);
        match insns[0] {
            Instruction::I32Const(v) => assert_eq!(v, 0x01234567),
            _ => panic!("not an I32Const"),
        }
    }

    #[test]
    fn push8() {
        let op = Opcode::PUSHn(
            8,
            u256::from(0x0123456789ABCDEF_u64),
            vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF],
        );
        let insns = encode_push(&op);
        assert_eq!(insns.len(), 1);
        match insns[0] {
            Instruction::I64Const(v) => assert_eq!(v, 0x0123456789ABCDEF),
            _ => panic!("not an I64Const"),
        }
    }

    #[test]
    fn push16() {
        let op = Opcode::PUSHn(
            16,
            u256::from(0x0123456789ABCDEFFEDCBA9876543210_u128),
            vec![
                0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
                0x32, 0x10,
            ],
        );
        let insns = encode_push(&op);
        assert_eq!(insns.len(), 2);
        match insns[0] {
            Instruction::I64Const(v) => assert_eq!(v as u64, 0xFEDCBA9876543210),
            _ => panic!("not an I64Const"),
        }
        match insns[1] {
            Instruction::I64Const(v) => assert_eq!(v, 0x0123456789ABCDEF),
            _ => panic!("not an I64Const"),
        }
    }

    #[test]
    fn push32() {
        let bs = vec![
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
            0x32, 0x10, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB,
            0xCC, 0xDD, 0xEE, 0xFF,
        ];
        let op = Opcode::PUSHn(32, u256::from_be_bytes(bs.clone().try_into().unwrap()), bs);
        let insns = encode_push(&op);
        assert_eq!(insns.len(), 4);
        match insns[0] {
            Instruction::I64Const(v) => assert_eq!(v as u64, 0x8899AABBCCDDEEFF),
            _ => panic!("not an I64Const"),
        }
        match insns[1] {
            Instruction::I64Const(v) => assert_eq!(v, 0x0011223344556677),
            _ => panic!("not an I64Const"),
        }
        match insns[2] {
            Instruction::I64Const(v) => assert_eq!(v as u64, 0xFEDCBA9876543210),
            _ => panic!("not an I64Const"),
        }
        match insns[3] {
            Instruction::I64Const(v) => assert_eq!(v, 0x0123456789ABCDEF),
            _ => panic!("not an I64Const"),
        }
    }
}
