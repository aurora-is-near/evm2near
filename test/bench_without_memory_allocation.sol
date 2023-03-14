// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.16;

contract Bench {
   
    function cpu_ram_soak_test(uint32 loop_limit) public pure {
        for (uint32 i=0; i < loop_limit; i++) {
            uint32 j = (i * 7 + 100500 / 2) % 100500;
            uint32 k = (i * 3) % 100500;
            uint32 abra = (i + 15) / 13;
            uint32 rrr = k + j;
            
        }
    }
}
