### This script assumes that evm2near is already compiled

import os

contracts = [
    'calc'
    # 'bench',
    # 'Collatz',
    # 'echo',
    # 'const'
]


def compile(name: str):
    os.system(f'./evm2near test/{name}.sol -o {name}.wasm -b near')


def copy(name: str):
    os.system(f'cp {name}.wasm tools/benchmark/{name}.wasm')


def remove(name: str):
    os.system(f'rm tools/benchmark/{name}.wasm')


def compile_contracts():
    for contract in contracts:
        compile(contract)


def copy_contracts():
    for contract in contracts:
        copy(contract)


def clean():
    for contract in contracts:
        remove(contract)


def run_bench():
    os.chdir('tools/benchmark')
    os.system('cargo run')
    os.chdir('../../')




import pandas as pd


if __name__ == "__main__":
    clean()
    compile_contracts()
    print("Contracts compiled")
    copy_contracts()
    print("Benchmark started")
    run_bench()
    print("Benchmark ended, see results in tools/benchmark/benchmark.csv")
    print("Clean started")
    clean()
    print("Clean ended")


    tables = []

    for file in os.listdir("tools/benchmark"):
        if file[-4] == ".csv":
            tables.append(pd.read_csv(file))


    res = pd.concat(tables, axis=0)
    tables.to_html("tools/benchmark/pages/index.html")
    html_file = tables.to_html()

    