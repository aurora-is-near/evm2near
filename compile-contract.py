import sys
import os

all = ' --no-check --inline-exports --inline-exports --generate-names '
nocheck = ' --no-check ' 
none = ''
wasm2wat_flags = all

recompile_compiler = False
recompile_evmlib = True

contract_name = sys.argv[1][5:-4] if len(sys.argv) > 1 else 'const'
print(contract_name) 

if recompile_compiler:
    os.system('make clean')
    os.system('make')

os.system(f'rm {contract_name}.wa*')
os.system(f'./evm2near test/{contract_name}.sol -o {contract_name}.wasm -b wasi')
os.system(f'wasm2wat {contract_name}.wasm -o {contract_name}.wat {wasm2wat_flags}')

if recompile_evmlib:
    os.system('cd lib/evmlib')
    os.system('make clean')
    os.system('rm lib/evmlib/evmlib.wa*')
    os.system('make')
    os.system('cd ../..')
    os.system(f'wasm2wat evmlib.wasm -o evmlib.wat {wasm2wat_flags}')

os.system(f'code --diff {contract_name}.wat evmlib.wat')
