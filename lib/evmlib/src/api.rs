// This is free and unencumbered software released into the public domain.

#[allow(unused_imports)]
use crate::{
    ops::{ENV, EVM},
    state::{Word, ZERO},
};

#[no_mangle]
pub unsafe fn _evm_init(_table_offset: u32, chain_id: u64, balance: u64) {
    #[cfg(feature = "near")]
    {
        // TODO
    }

    #[cfg(not(feature = "near"))]
    {
        let mut args = std::env::args();

        // Remove fluff from the command-line arguments:
        let mut arg = args.next();
        let mut arg_pos = 0;
        loop {
            match &arg {
                None => break, // no more arguments
                Some(s) => {
                    if arg_pos == 0 && (s.ends_with(".wasm") || s.ends_with(".wasi")) {
                        // consume the program name
                    } else {
                        match s.as_str() {
                            "--" => {
                                arg = args.next(); // start of actual arguments
                                break;
                            }
                            "--func" | "--invoke" => _ = args.next(), // skip interpreter options
                            _ => break,                               // start of actual arguments
                        }
                    }
                }
            }
            arg = args.next();
            arg_pos += 1;
        }

        ENV.call_data = match arg {
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
        //eprintln!("_evm_init: call_data={:?} call_value={:?}", ENV.call_data, EVM.call_value);
    }

    EVM.chain_id = Word::from(chain_id);
    EVM.self_balance = Word::from(balance);
}

#[no_mangle]
pub unsafe fn _evm_call(selector: u32) {
    #[cfg(feature = "near")]
    {
        // TODO
    }
    #[cfg(any(not(feature = "near"), test))]
    {
        ENV.call_data.splice(0..0, selector.to_be_bytes());
        //eprintln!("_evm_call: call_data={:?} call_value={:?}", ENV.call_data, EVM.call_value);
        // TODO
    }
}

#[no_mangle]
pub unsafe fn _evm_pop_u32() -> u32 {
    EVM.stack.pop().as_u32()
}
