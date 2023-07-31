// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.16;

contract Bench {
    function make_great_cfg(uint32 x) public pure {
        if (x % 2 == 0) {
            odd(x / 2);
        } else {
            even(x + 5);
        }
    }

    function odd(uint32 x) public pure {
        if (x % 3 == 0) {
            divide3(x + 15);
        } else {
            if (x % 3 == 1) {
                divide3get1(x + 14);
            } else {
                divide3get2(x + 100);
            }
        }
    }

    function even(uint32 x) public pure {
        if (x % 3 == 0) {
            divide3(x + 1);
        } else {
            if (x % 3 == 1) {
                divide3get1(x + 184);
            } else {
                divide3get2(x + 1000);
            }
        }
    }

    function divide3(uint32 x) public pure {
        for (uint32 i = 0; i < x; i++) {
            if (i % 2 == 0) {
                uint32 y = x + 5;
            } else {
                uint32 y = x + 3;
            }
        }
    }

    function divide3get1(uint32 x) public pure {
        for (uint32 i = 0; i < x; i++) {
            if (i % 2 == 0) {
                uint32 y = x + 54;
            } else {
                uint32 y = x + 34;
            }
        }
    }

    function divide3get2(uint32 x) public pure {
        for (uint32 i = 0; i < x; i++) {
            if (i % 2 == 0) {
                uint32 y = x + 53;
            } else {
                uint32 y = x + 31;
            }
        }
    }
}
