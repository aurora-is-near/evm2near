// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.16;


contract Bench {

    /// start from calling this function
    function make_great_cfg(int x) public pure returns(int) {
        if (x % 2 == 0) {
            return odd(x / 2);
        } else {
            return even(x + 3);
        }
    }

    function odd(int x) public pure returns(int) {
        if (x % 3 == 0) {
            return divide3(x + 4);
        } else {
            if (x % 3 == 1) {
                return divide3get1(x + 5);
            } else {
                return divide3get2(x + 6);
            }
        }
    }

    function even(int x) public pure returns(int) {
        if (x % 3 == 0) {
            return divide3(x + 7);
        } else {
            if (x % 3 == 1) {
                return divide3get1(x + 8);
            } else {
                return divide3get2(x + 9);
            }
        }
    }

    function divide3(int x) public pure returns(int) {
        int yy = 0;
        for (int i = 0; i < x; i++) {
            if (i % 2 == 0) {
                int y = x + 10;
                yy = yy + y;
            } else {
                int y = x + 20;
                yy = yy + y;
            }
            
        }
        return yy;
    }

    function divide3get1(int x) public pure returns(int) {
        int  yy = 1;
        for (int i = 0; i < x; i++) {
            if (i % 2 == 0) {
                int y = x + 30;
                yy = yy + y;
            } else {
                int y = x + 40;
                yy = yy + y;
            }
        }
        return yy;
    }

    function divide3get2(int x) public pure returns(int) {
        int yy = 2;
        for (int i = 0; i < x; i++) {
            if (i % 2 == 0) {
                int y = x + 50;
                yy = yy + y;
            } else {
                int y = x + 60;
                yy = yy + y;
            }
        }
        return yy;
    }
}