### This script assumes that evm2near is already compiled

import os
import subprocess


def compile(name: str):
    os.system(f'./evm2near test/{name}.sol -o {name}.wasm -b wasi')


def compile_contracts():
    compile('calc')
    compile('bench')


def test_calc():
    res = subprocess.run(["wasmtime", "--allow-unknown-exports", "calc.wasm", "--invoke", "multiply", "--", "{\"a\":6, \"b\": 7}"],
                          stdout=subprocess.PIPE)
    assert res.stdout.decode('utf8') == """Result: Success
7b226f7574707574223a34322c22737461747573223a2253554343455353227d
{"output":42,"status":"SUCCESS"}
"""
    assert res.returncode == 0
    res = subprocess.run(["wasmtime", "--allow-unknown-exports", "calc.wasm", "--invoke", "multiply", "--", "{\"a\":-3, \"b\": -2}"],
                          stdout=subprocess.PIPE)
    assert res.stdout.decode('utf8') == """Result: Success
7b226f7574707574223a362c22737461747573223a2253554343455353227d
{"output":6,"status":"SUCCESS"}
"""
    assert res.returncode == 0
    print("Calc tests passed")


def test_bench():
    res = subprocess.run(["wasmtime", "--allow-unknown-exports", "bench.wasm", "--invoke", "cpu_ram_soak_test", "--", "{\"loop_limit\": 100000}"],
                          stdout=subprocess.PIPE)
    assert res.returncode == 0
    print("Bench test with 100000 iterations passed succesfully")


def test_contracts():
    test_calc()
    test_bench()


if __name__ == "__main__":
    compile_contracts()
    test_contracts()