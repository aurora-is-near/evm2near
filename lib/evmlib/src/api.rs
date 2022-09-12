// This is free and unencumbered software released into the public domain.

#[allow(unused_imports)]
use crate::{
    ops::{ENV, EVM},
    state::{Word, ZERO},
};

#[no_mangle]
pub unsafe fn _init_evm(_table_offset: u32, chain_id: u64, balance: u64) {
    #[cfg(feature = "near")]
    {
        // TODO
    }
    #[cfg(not(feature = "near"))]
    {
        let mut args = std::env::args();
        // TODO: look for "--""
        let _ = args.next(); // consume the program name
        ENV.call_data = match args.next() {
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
    EVM.chain_id = Word::from(chain_id);
    EVM.self_balance = Word::from(balance);
}

#[no_mangle]
pub unsafe fn _prepare(selector: u32) {
    #[cfg(feature = "near")]
    {
        // TODO
    }
    #[cfg(any(not(feature = "near"), test))]
    {
        ENV.call_data.splice(0..0, selector.to_be_bytes());
        // TODO
    }
}

#[no_mangle]
pub unsafe fn _pop_u32() -> u32 {
    EVM.stack.pop().as_u32()
}
