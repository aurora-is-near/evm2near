from os import system
import json


def inputize(d : dict) -> str:
    return '\'' + json.dumps(d) + '\''


tmpfile = 'tmp0801371794747427242472.txt'
deployed = {}


def deploy(file : str):
    system(f'./evm2near {file} -o test.wasm -b near')
    print(f'Contract {file} compiled succesfully')
    system(f'near --networkId testnet dev-deploy test.wasm >{tmpfile}')
    with open(tmpfile, 'r') as f:
        s = f.readline()
        accountId = s[s.find('dev'):s.find(',')]
    print(f'Contract {file} deployed to testnet with accountId={accountId}')
    deployed[file] = accountId


def TestOne(file : str, method : str, input : dict, expected_output : str):
    if file not in deployed:
        deploy(file)
    accountId = deployed[file]
    system(f'near call --account-id {accountId} {accountId}' + f' {method}  {inputize(input)}' + f' >{tmpfile}')
    with open(tmpfile, 'r') as f:
        s = f.readlines()[-1]
        print(f'Call to {file} with args={method}{inputize(input)} return {s[:-1]}')
        real_output = s[s.find(': ') + 2:s.find(',')]
    assert real_output == expected_output


def All():
    TestOne('test/calc.sol', 'multiply', {"a" : 6, "b" : 7}, '42')
    TestOne('test/calc.sol', 'multiply', {"a" : 0, "b" : 7}, '0')
    TestOne('test/calc.sol', 'multiply', {"a" : 6, "b" : -7}, '-42')
    TestOne('test/calc.sol', 'multiply', {"a" : -6, "b" : -1}, '6')
    TestOne('test/echo.sol', 'echo', {"x" : 333}, '333')
    TestOne('test/const.sol', 'value', {}, '42')


def main():
    system(f'touch {tmpfile}')
    system('make')
    All()
    system(f'rm {tmpfile}')

main()
