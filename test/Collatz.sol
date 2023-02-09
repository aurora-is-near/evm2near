// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

// A contract implementing the function from the famous Collatz conjecture.
// See https://en.wikipedia.org/wiki/Collatz_conjecture
contract Collatz {
    // The number of iterations to reach 1. Could never return if the conjecture is false.
    // See https://oeis.org/A006577 for the first few values
    function totalStoppingTime(uint256 n) public pure returns (uint256) {
        uint256 count = 0;
        while (n > 1) {
            n = f(n);
            count += 1;
        }
        return count;
    }

    // An alternate implementation of totalStoppingTime where we use a recursive function.
    function recursiveTotalStoppingTime(uint256 n) public pure returns (uint256) {
        return recursionHelper(n, 0);
    }

    // The function used in the Collatz conjecture dynamical system.
    function f(uint256 n) private pure returns (uint256) {
        uint256 m;
        if (n % 2 == 0) {
            m = n / 2;
        } else {
            m = 3 * n + 1;
        }
        return m;
    }

    // A helper function so that the recursion is in the tail-call position.
    function recursionHelper(uint256 n, uint256 acc) private pure returns (uint256) {
        if (n <= 1) {
            return acc;
        }

        return recursionHelper(f(n), acc + 1);
    }
}
