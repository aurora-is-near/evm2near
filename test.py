from os import system
import json
import subprocess


def inputize(d : dict) -> str:
    return '\'' + json.dumps(d) + '\''


deployed = {}


def deploy(file : str):
    system(f'./evm2near {file} -o test.wasm -b near')
    print(f'Contract {file} compiled succesfully')
    out = subprocess.run(["near",
                                "--networkId",
                                "testnet",
                                "dev-deploy",
                                "test.wasm"],
                               capture_output=True).stdout.decode('ascii')
    accountId = out[out.find('dev'):out.find(',')]
    print(f'Contract {file} deployed to testnet with accountId={accountId}')
    deployed[file] = accountId


def TestOne(file : str, method : str, input : dict, expected_output : str):
    if file not in deployed:
        deploy(file)
    accountId = deployed[file]
    out = subprocess.run(["near",
                          "call",
                          "--account-id",
                          f"{accountId}",
                          f"{accountId}",
                          f"{method}",
                          f"{inputize(input)}"],
                         capture_output=True).stdout.decode('ascii')[:-1]     # rm \n in the end
    lastline = out[out.rfind('\n'):]
    print(f'Call to {file} with args={method}{inputize(input)} return {lastline}')
    real_output = lastline[lastline.find(': ') + 2:lastline.find(',')]
    assert real_output == expected_output


def All():
    TestOne('test/calc.sol', 'multiply', {"a" : 6, "b" : 7}, '42')
    TestOne('test/calc.sol', 'multiply', {"a" : 0, "b" : 7}, '0')
    TestOne('test/calc.sol', 'multiply', {"a" : 6, "b" : -7}, '-42')
    TestOne('test/calc.sol', 'multiply', {"a" : -6, "b" : -1}, '6')
    TestOne('test/echo.sol', 'echo', {"x" : 333}, '333')
    TestOne('test/const.sol', 'value', {}, '42')


def main():
    system('make')
    All()
    system('rm test.wasm')


if __name__ == '__main__':
    main()
