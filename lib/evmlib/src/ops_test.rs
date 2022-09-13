// This is free and unencumbered software released into the public domain.

#[cfg(test)]
mod tests {
    use crate::env::{Env, ExitStatus};
    use crate::ops::*;
    use crate::state::*;
    use ux::*;

    #[test]
    fn test_stop() {
        unsafe {
            EVM.reset();
            ENV.reset();
            stop();
            assert_eq!(ENV.exit_status, Some(ExitStatus::Success));
            assert!(ENV.return_data.is_empty());
        }
    }

    #[test]
    fn test_add() {
        unsafe {
            EVM.reset();
            push1(6);
            push1(7);
            add();
            assert_eq!(EVM.stack.peek(), 13);
        }
    }

    #[test]
    fn test_mul() {
        unsafe {
            EVM.reset();
            push1(6);
            push1(7);
            mul();
            assert_eq!(EVM.stack.peek(), 42);
        }
    }

    #[test]
    fn test_sub() {
        unsafe {
            EVM.reset();
            push1(6);
            push1(7);
            sub();
            assert_eq!(EVM.stack.peek(), 1);
        }
    }

    #[test]
    fn test_div() {
        unsafe {
            EVM.reset();
            push1(6);
            push1(42);
            div();
            assert_eq!(EVM.stack.peek(), 7);
        }
    }

    #[test]
    fn test_sdiv() {}

    #[test]
    fn test_mod() {}

    #[test]
    fn test_smod() {}

    #[test]
    fn test_addmod() {
        unsafe {
            EVM.reset();
            push1(25);
            push1(37);
            push1(11);
            addmod();
            assert_eq!(EVM.stack.peek(), 23);
        }
    }

    #[test]
    fn test_mulmod() {
        unsafe {
            EVM.reset();
            push1(5);
            push1(6);
            push1(7);
            mulmod();
            assert_eq!(EVM.stack.peek(), 2);
        }
    }

    #[test]
    fn test_exp() {
        unsafe {
            EVM.reset();
            push1(4);
            push1(2);
            exp();
            assert_eq!(EVM.stack.peek(), 16);
        }
    }

    #[test]
    fn test_signextend() {
        // Test cases from https://www.evm.codes/
        unsafe {
            EVM.reset();
            push1(0xFF);
            push1(0x00);
            signextend();
            assert_eq!(EVM.stack.peek(), Word::MAX);
        }
        unsafe {
            EVM.reset();
            push1(0x7F);
            push1(0x00);
            signextend();
            assert_eq!(EVM.stack.peek(), 0x7F);
        }

        // Additional test case
        unsafe {
            EVM.reset();
            EVM.stack.push(
                "0x1886E5F0ABB04994B1D20310DCBE15760932963A40621B97C2AEC12652C7480".hex_int(),
            );
            push1(0x10);
            signextend();
            assert_eq!(
                EVM.stack.peek(),
                "0x5760932963A40621B97C2AEC12652C7480".hex_int()
            );
        }
    }

    #[test]
    fn test_lt() {}

    #[test]
    fn test_gt() {}

    #[test]
    fn test_slt() {}

    #[test]
    fn test_sgt() {}

    #[test]
    fn test_eq() {}

    #[test]
    fn test_iszero() {}

    #[test]
    fn test_and() {}

    #[test]
    fn test_or() {}

    #[test]
    fn test_xor() {}

    #[test]
    fn test_not() {}

    #[test]
    fn test_byte() {
        // Test case from https://www.evm.codes/
        unsafe {
            EVM.reset();
            push1(0xFF);
            push1(0x1F);
            byte();
            assert_eq!(EVM.stack.peek(), "0xFF".hex_int(),);
        }
        unsafe {
            EVM.reset();
            push2(0xFF00);
            push1(0x1E);
            byte();
            assert_eq!(EVM.stack.peek(), "0xFF".hex_int(),);
        }
    }

    #[test]
    fn test_shl() {}

    #[test]
    fn test_shr() {}

    #[test]
    fn test_sar() {
        // Test case from https://www.evm.codes/
        unsafe {
            EVM.reset();
            push1(0x02);
            push1(0x01);
            sar();
            assert_eq!(EVM.stack.peek(), "0x01".hex_int(),);
        }
        unsafe {
            EVM.reset();
            EVM.stack.push(
                "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0".hex_int(),
            );
            push1(0x04);
            sar();
            assert_eq!(
                EVM.stack.peek(),
                "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".hex_int(),
            );
        }
        unsafe {
            EVM.reset();
            EVM.stack.push(
                "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF10".hex_int(),
            );
            push1(0x04);
            sar();
            assert_eq!(
                EVM.stack.peek(),
                "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF1".hex_int(),
            );
        }
    }

    #[test]
    fn test_sha3() {
        use ::sha3::{Digest, Keccak256};

        // Test hash of empty input
        unsafe {
            EVM.reset();
            push1(0x00);
            push1(0x00);
            sha3();
            assert_eq!(
                EVM.stack.peek(),
                Word::from_be_bytes(Keccak256::digest([]).try_into().unwrap()),
            );
        }

        // Test case from https://www.evm.codes/
        unsafe {
            EVM.reset();
            EVM.memory.store_slice(0x00, &[0xFFu8; 4]);
            push1(0x04);
            push1(0x00);
            sha3();
            assert_eq!(
                EVM.stack.peek(),
                "0x29045A592007D0C246EF02C2223570DA9522D0CF0F73282C79A1BC8F0BB2C238".hex_int(),
            );
        }
    }

    #[test]
    fn test_address() {
        let mock_address = [0xABu8; 20];
        unsafe {
            ENV.address = mock_address;
            EVM.reset();
            address();
            assert_eq!(
                EVM.stack.peek(),
                "0xABABABABABABABABABABABABABABABABABABABAB".hex_int()
            );
        }
    }

    #[test]
    fn test_balance() {
        let mock_address = hex::decode("2fAD5818188D71A1d6A4868d352E69f239AFdee9")
            .unwrap()
            .try_into()
            .unwrap();
        let mock_balance = 132456;
        unsafe {
            ENV.address = mock_address;
            EVM.reset();
            EVM.self_balance = Word::from(mock_balance);
            EVM.stack
                .push("0x2fAD5818188D71A1d6A4868d352E69f239AFdee9".hex_int());
            balance();
            assert_eq!(EVM.stack.peek(), mock_balance);

            // Balances other than self are zero
            EVM.stack
                .push("0x0000000000000DEADBEEF0000000000000000000".hex_int());
            balance();
            assert_eq!(EVM.stack.peek(), ZERO);
        }
    }

    #[test]
    fn test_origin() {
        let mock_address = [0xEFu8; 20];
        unsafe {
            ENV.origin = mock_address;
            EVM.reset();
            origin();
            assert_eq!(
                EVM.stack.peek(),
                "0xEFEFEFEFEFEFEFEFEFEFEFEFEFEFEFEFEFEFEFEF".hex_int()
            );
        }
    }

    #[test]
    fn test_caller() {
        let mock_address = [0xCDu8; 20];
        unsafe {
            ENV.caller = mock_address;
            EVM.reset();
            caller();
            assert_eq!(
                EVM.stack.peek(),
                "0xCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCD".hex_int()
            );
        }
    }

    #[test]
    fn test_callvalue() {}

    #[test]
    fn test_calldataload() {
        // test cases from https://www.evm.codes/
        unsafe {
            EVM.reset();
            ENV.call_data = vec![255u8; 32];
            push1(0x00);
            calldataload();
            assert_eq!(EVM.stack.peek(), Word::MAX,);
        }
        unsafe {
            EVM.reset();
            ENV.call_data = vec![255u8; 32];
            push1(0x1F);
            calldataload();
            assert_eq!(
                EVM.stack.peek(),
                "0xFF00000000000000000000000000000000000000000000000000000000000000".hex_int(),
            );
        }
    }

    #[test]
    fn test_calldatasize() {}

    #[test]
    fn test_calldatacopy() {
        // test cases from https://www.evm.codes/
        unsafe {
            EVM.reset();
            ENV.call_data = vec![255u8; 32];
            push1(0x20);
            push1(0x00);
            push1(0x00);
            calldatacopy();
            assert_eq!(&EVM.memory.bytes, &[255u8; 32]);

            push1(0x08);
            push1(0x1F);
            push1(0x00);
            calldatacopy();
            assert_eq!(
                EVM.memory.bytes,
                hex::decode("FF00000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
                    .unwrap()
            );
        }
    }

    #[test]
    fn test_codesize() {}

    #[test]
    fn test_codecopy() {
        // test cases from https://www.evm.codes/
        unsafe {
            EVM.reset();
            EVM.code =
                hex::decode("7DFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF7F")
                    .unwrap();
            push1(0x20);
            push1(0x00);
            push1(0x00);
            codecopy();
            assert_eq!(&EVM.memory.bytes, &EVM.code);

            push1(0x08);
            push1(0x1F);
            push1(0x00);
            codecopy();
            assert_eq!(
                EVM.memory.bytes,
                hex::decode("7F00000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF7F")
                    .unwrap()
            );
        }
    }

    #[test]
    fn test_gasprice() {}

    #[test]
    fn test_extcodesize() {}

    #[test]
    fn test_extcodecopy() {}

    #[test]
    fn test_returndatasize() {}

    #[test]
    fn test_returndatacopy() {}

    #[test]
    fn test_extcodehash() {}

    #[test]
    fn test_blockhash() {}

    #[test]
    fn test_coinbase() {}

    #[test]
    fn test_timestamp() {
        let mock_timestamp = 1662652905;
        unsafe {
            ENV.timestamp = mock_timestamp;
            EVM.reset();
            timestamp();
            assert_eq!(EVM.stack.peek(), mock_timestamp as u128);
        }
    }

    #[test]
    fn test_number() {
        let block_height = 2718;
        unsafe {
            ENV.block_height = block_height;
            EVM.reset();
            number();
            assert_eq!(EVM.stack.peek(), block_height as u128);
        }
    }

    #[test]
    fn test_difficulty() {}

    #[test]
    fn test_gaslimit() {}

    #[test]
    fn test_chainid() {
        let aurora_mainnet = 1313161554;
        unsafe {
            EVM.reset();
            EVM.chain_id = Word::from(aurora_mainnet);
            chainid();
            assert_eq!(EVM.stack.peek(), aurora_mainnet);
        }
    }

    #[test]
    fn test_selfbalance() {
        let balance = 3141592653589793238;
        unsafe {
            EVM.reset();
            EVM.self_balance = Word::from(balance);
            selfbalance();
            assert_eq!(EVM.stack.peek(), balance);
        }
    }

    #[test]
    fn test_basefee() {}

    #[test]
    fn test_pop() {
        unsafe {
            EVM.reset();
            push1(42);
            assert_eq!(EVM.stack.depth, 1);
            pop();
            assert_eq!(EVM.stack.depth, 0);
        }
    }

    #[test]
    fn test_mload() {
        unsafe {
            EVM.reset();
            push1(0);
            mload();
            assert_eq!(EVM.stack.peek(), 0);
        }
    }

    #[test]
    fn test_mstore() {
        unsafe {
            EVM.reset();
            push1(42);
            push1(0);
            mstore();
            assert_eq!(EVM.memory.load_word(0), 42);
        }
    }

    #[test]
    fn test_mstore8() {
        unsafe {
            EVM.reset();
            push1(42);
            push1(31);
            mstore8();
            assert_eq!(EVM.memory.load_word(0), 42);
        }
    }

    #[test]
    fn test_sload() {
        unsafe {
            EVM.reset();
            ENV.storage_write(Word::from(42u8), Word::from(123u8));
            push1(42);
            sload();
            assert_eq!(EVM.stack.peek(), 123);
        }
    }

    #[test]
    fn test_sstore() {
        unsafe {
            EVM.reset();
            push1(6);
            push1(7);
            sstore();
            assert_eq!(EVM.stack.depth, 0);
            assert_eq!(ENV.storage_read(Word::from(7u8)), 6);
        }
    }

    #[test]
    fn test_jump() {}

    #[test]
    fn test_jumpi() {}

    #[test]
    fn test_pc() {}

    #[test]
    fn test_msize() {}

    #[test]
    fn test_gas() {}

    #[test]
    fn test_jumpdest() {}

    #[test]
    fn test_push1() {
        unsafe {
            EVM.reset();
            push1(0x12u8);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x12u8));
        }
    }

    #[test]
    fn test_push2() {
        unsafe {
            EVM.reset();
            push2(0x1234u16);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x1234u16));
        }
    }

    #[test]
    fn test_push3() {
        unsafe {
            EVM.reset();
            push3(u24::try_from(0x123456u32).unwrap());
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x123456u32));
        }
    }

    #[test]
    fn test_push4() {
        unsafe {
            EVM.reset();
            push4(0x12345678_u32);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x12345678u32));
        }
    }

    #[test]
    fn test_push5() {
        unsafe {
            EVM.reset();
            push5(u40::try_from(0x123456789Au64).unwrap());
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x123456789Au64));
        }
    }

    #[test]
    fn test_push6() {
        unsafe {
            EVM.reset();
            push6(u48::try_from(0x123456789ABCu64).unwrap());
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x123456789ABCu64));
        }
    }

    #[test]
    fn test_push7() {
        unsafe {
            EVM.reset();
            push7(u56::try_from(0x123456789ABCDEu64).unwrap());
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x123456789ABCDEu64));
        }
    }

    #[test]
    fn test_push8() {
        unsafe {
            EVM.reset();
            push8(0x123456789ABCDEF0_u64);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x123456789ABCDEF0u64));
        }
    }

    #[test]
    fn test_push9() {
        unsafe {
            EVM.reset();
            push9(0x123456789ABCDEF011_u128);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x123456789ABCDEF011u128));
        }
    }

    #[test]
    fn test_push10() {
        unsafe {
            EVM.reset();
            push10(0x123456789ABCDEF01122_u128);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x123456789ABCDEF01122u128));
        }
    }

    #[test]
    fn test_push11() {
        unsafe {
            EVM.reset();
            push11(0x123456789ABCDEF0112233_u128);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], Word::from(0x123456789ABCDEF0112233u128));
        }
    }

    #[test]
    fn test_push12() {
        unsafe {
            EVM.reset();
            push12(0x123456789ABCDEF011223344_u128);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(
                EVM.stack.slots[0],
                Word::from(0x123456789ABCDEF011223344u128)
            );
        }
    }

    #[test]
    fn test_push13() {
        unsafe {
            EVM.reset();
            push13(0x123456789ABCDEF01122334455_u128);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(
                EVM.stack.slots[0],
                Word::from(0x123456789ABCDEF01122334455u128)
            );
        }
    }

    #[test]
    fn test_push14() {
        unsafe {
            EVM.reset();
            push14(0x123456789ABCDEF0112233445566_u128);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(
                EVM.stack.slots[0],
                Word::from(0x123456789ABCDEF0112233445566u128)
            );
        }
    }

    #[test]
    fn test_push15() {
        unsafe {
            EVM.reset();
            push15(0x123456789ABCDEF011223344556677_u128);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(
                EVM.stack.slots[0],
                Word::from(0x123456789ABCDEF011223344556677u128)
            );
        }
    }

    #[test]
    fn test_push16() {
        unsafe {
            EVM.reset();
            push16(0x123456789ABCDEF01122334455667788_u128);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(
                EVM.stack.slots[0],
                Word::from(0x123456789ABCDEF01122334455667788u128)
            );
        }
    }

    #[test]
    fn test_push17() {}

    #[test]
    fn test_push18() {}

    #[test]
    fn test_push19() {}

    #[test]
    fn test_push20() {}

    #[test]
    fn test_push21() {}

    #[test]
    fn test_push22() {}

    #[test]
    fn test_push23() {}

    #[test]
    fn test_push24() {
        unsafe {
            EVM.reset();
            push24(0x99AABBCCDDEEFF00, 0x1122334455667788, 0x123456789ABCDEF0);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(
                EVM.stack.slots[0],
                Word::from_str_hex("0x123456789ABCDEF0112233445566778899AABBCCDDEEFF00").unwrap()
            );
        }
    }

    #[test]
    fn test_push25() {}

    #[test]
    fn test_push26() {}

    #[test]
    fn test_push27() {}

    #[test]
    fn test_push28() {}

    #[test]
    fn test_push29() {}

    #[test]
    fn test_push30() {}

    #[test]
    fn test_push31() {}

    #[test]
    fn test_push32() {
        unsafe {
            EVM.reset();
            push32(
                0x99AABBCCDDEEFF00,
                0x1122334455667788,
                0x123456789ABCDEF0,
                0xCAFEBABEDECAFBAD,
            );
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(
                EVM.stack.slots[0],
                Word::from_str_hex(
                    "0xCAFEBABEDECAFBAD123456789ABCDEF0112233445566778899AABBCCDDEEFF00"
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn test_dup1() {
        unsafe {
            EVM.reset();
            push1(42);
            assert_eq!(EVM.stack.depth, 1);
            assert_eq!(EVM.stack.slots[0], 42);
            dup1();
            assert_eq!(EVM.stack.depth, 2);
            assert_eq!(EVM.stack.slots[1], 42);
        }
    }

    #[test]
    fn test_dup2() {
        unsafe {
            EVM.reset();
            push1(34);
            push1(12);
            assert_eq!(EVM.stack.depth, 2);
            assert_eq!(EVM.stack.slots[0], 34);
            assert_eq!(EVM.stack.slots[1], 12);
            dup2();
            assert_eq!(EVM.stack.depth, 3);
            assert_eq!(EVM.stack.slots[0], 34);
            assert_eq!(EVM.stack.slots[1], 12);
            assert_eq!(EVM.stack.slots[2], 34);
        }
    }

    #[test]
    fn test_dup3() {}

    #[test]
    fn test_dup4() {}

    #[test]
    fn test_dup5() {}

    #[test]
    fn test_dup6() {}

    #[test]
    fn test_dup7() {}

    #[test]
    fn test_dup8() {}

    #[test]
    fn test_dup9() {}

    #[test]
    fn test_dup10() {}

    #[test]
    fn test_dup11() {}

    #[test]
    fn test_dup12() {}

    #[test]
    fn test_dup13() {}

    #[test]
    fn test_dup14() {}

    #[test]
    fn test_dup15() {}

    #[test]
    fn test_dup16() {}

    #[test]
    fn test_swap1() {
        unsafe {
            EVM.reset();
            push1(34);
            push1(12);
            assert_eq!(EVM.stack.depth, 2);
            assert_eq!(EVM.stack.slots[0], 34);
            assert_eq!(EVM.stack.slots[1], 12);
            swap1();
            assert_eq!(EVM.stack.depth, 2);
            assert_eq!(EVM.stack.slots[0], 12);
            assert_eq!(EVM.stack.slots[1], 34);
        }
    }

    #[test]
    fn test_swap2() {}

    #[test]
    fn test_swap3() {}

    #[test]
    fn test_swap4() {}

    #[test]
    fn test_swap5() {}

    #[test]
    fn test_swap6() {}

    #[test]
    fn test_swap7() {}

    #[test]
    fn test_swap8() {}

    #[test]
    fn test_swap9() {}

    #[test]
    fn test_swap10() {}

    #[test]
    fn test_swap11() {}

    #[test]
    fn test_swap12() {}

    #[test]
    fn test_swap13() {}

    #[test]
    fn test_swap14() {}

    #[test]
    fn test_swap15() {}

    #[test]
    fn test_swap16() {}

    #[test]
    fn test_log0() {
        let test_data = b"hello_world_0";
        let test_address = [0x12; 20];
        unsafe {
            EVM.reset();
            ENV.address = test_address;
            EVM.memory.bytes = test_data.to_vec();
            push1(test_data.len() as u8);
            push1(0);
            log0();
            let log = ENV.logs.first().unwrap();
            assert_eq!(
                log,
                &crate::env::mock::OwnedEvmLog {
                    address: test_address,
                    topics: Vec::new(),
                    data: test_data.to_vec()
                }
            )
        }
    }

    #[test]
    fn test_log1() {
        let test_data = b"hello_world_1";
        let test_address = [0x34; 20];
        let topic = "0xdeadbeef".hex_int();
        unsafe {
            EVM.reset();
            ENV.reset();
            ENV.address = test_address;
            EVM.memory.bytes = test_data.to_vec();
            EVM.stack.push(topic);
            push1(test_data.len() as u8);
            push1(0);
            log1();
            let log = ENV.logs.first().unwrap();
            assert_eq!(
                log,
                &crate::env::mock::OwnedEvmLog {
                    address: test_address,
                    topics: vec![topic],
                    data: test_data.to_vec()
                }
            )
        }
    }

    #[test]
    fn test_log2() {
        let test_data = b"hello_world_2";
        let test_address = [0x56; 20];
        let topic1 = "0xdeadbeef".hex_int();
        let topic2 = "0x13246798".hex_int();
        unsafe {
            EVM.reset();
            ENV.reset();
            ENV.address = test_address;
            EVM.memory.bytes = test_data.to_vec();
            EVM.stack.push(topic2);
            EVM.stack.push(topic1);
            push1(test_data.len() as u8);
            push1(0);
            log2();
            let log = ENV.logs.first().unwrap();
            assert_eq!(
                log,
                &crate::env::mock::OwnedEvmLog {
                    address: test_address,
                    topics: vec![topic1, topic2],
                    data: test_data.to_vec()
                }
            )
        }
    }

    #[test]
    fn test_log3() {
        let test_data = b"hello_world_3";
        let test_address = [0x78; 20];
        let topic1 = "0xdeadbeef".hex_int();
        let topic2 = "0x13246798".hex_int();
        let topic3 = "0xabcdef01".hex_int();
        unsafe {
            EVM.reset();
            ENV.reset();
            ENV.address = test_address;
            EVM.memory.bytes = test_data.to_vec();
            EVM.stack.push(topic3);
            EVM.stack.push(topic2);
            EVM.stack.push(topic1);
            push1(test_data.len() as u8);
            push1(0);
            log3();
            let log = ENV.logs.first().unwrap();
            assert_eq!(
                log,
                &crate::env::mock::OwnedEvmLog {
                    address: test_address,
                    topics: vec![topic1, topic2, topic3],
                    data: test_data.to_vec()
                }
            )
        }
    }

    #[test]
    fn test_log4() {
        let test_data = b"hello_world_4";
        let test_address = [0x78; 20];
        let topic1 = "0xdeadbeef".hex_int();
        let topic2 = "0x13246798".hex_int();
        let topic3 = "0xabcdef01".hex_int();
        let topic4 = "0xabed".hex_int();
        unsafe {
            EVM.reset();
            ENV.reset();
            ENV.address = test_address;
            EVM.memory.bytes = test_data.to_vec();
            EVM.stack.push(topic4);
            EVM.stack.push(topic3);
            EVM.stack.push(topic2);
            EVM.stack.push(topic1);
            push1(test_data.len() as u8);
            push1(0);
            log4();
            let log = ENV.logs.first().unwrap();
            assert_eq!(
                log,
                &crate::env::mock::OwnedEvmLog {
                    address: test_address,
                    topics: vec![topic1, topic2, topic3, topic4],
                    data: test_data.to_vec()
                }
            )
        }
    }

    #[test]
    fn test_create() {}

    #[test]
    fn test_call() {}

    #[test]
    fn test_callcode() {}

    #[test]
    fn test_return() {
        let test_data = b"hello_return";
        unsafe {
            EVM.reset();
            ENV.reset();
            EVM.memory.bytes = test_data.to_vec();
            push1(test_data.len() as u8);
            push1(0);
            r#return();
            assert_eq!(ENV.exit_status, Some(ExitStatus::Success));
            assert_eq!(&ENV.return_data, test_data);
        }
    }

    #[test]
    fn test_delegatecall() {}

    #[test]
    fn test_create2() {}

    #[test]
    fn test_staticcall() {}

    #[test]
    fn test_revert() {
        let test_data = b"hello_revert";
        unsafe {
            EVM.reset();
            ENV.reset();
            EVM.memory.bytes = test_data.to_vec();
            push1(test_data.len() as u8);
            push1(0);
            revert();
            assert_eq!(ENV.exit_status, Some(ExitStatus::Revert));
            assert_eq!(&ENV.return_data, test_data);
        }
    }

    #[test]
    fn test_invalid() {
        unsafe {
            EVM.reset();
            ENV.reset();
            invalid();
            assert_eq!(ENV.exit_status, Some(ExitStatus::Revert));
            assert!(ENV.return_data.is_empty());
        }
    }

    #[test]
    fn test_selfdestruct() {}

    /// Helper trait to allow writing `.hex_int()` on hex strings in tests to convert
    /// them into 256-bit integers.
    trait HexInt {
        fn hex_int(self) -> Word;
    }
    impl<'a> HexInt for &'a str {
        fn hex_int(self) -> Word {
            Word::from_str_hex(self).unwrap()
        }
    }
}
