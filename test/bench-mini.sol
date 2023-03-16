// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.16;

contract Bench {
    function cpu_ram_soak_test(uint32 loop_limit) public pure {
        uint8[100] memory buf;
        uint32 len = 100;
        for (uint32 i=0; i < loop_limit; i++) {
            uint32 j = (i * 7 + len / 2) % len;
            uint32 k = (i * 3) % len;
            uint8 tmp = buf[k];
            buf[k] = buf[j];
            buf[j] = tmp;
        }
    }
}