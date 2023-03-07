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


def check_ci():
    with open("tools/benchmark/pages/index.html", "w") as html:
        lines = [
"<!DOCTYPE html>"
"<html>",
"  <head>",
"    <meta charset=\"utf-8\">",
"    <title>CSV File Viewer</title>",
"    <script src=\"https://d3js.org/d3.v6.min.js\"></script>",
"  </head>",
"  <body>",
"    <h1>CSV File Viewer Checking CI</h1>",
"    <table id=\"csvTable\">",
"      <thead>",
"        <tr></tr>",
"      </thead>",
"      <tbody></tbody>",
"    </table>",
"  </body>",
"</html>"]
        html.writelines(lines)


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


    check_ci()
    