### This script assumes that evm2near is already compiled

import os


print(os.listdir("tools/benchmark/inputs"))

contracts = list(map(lambda x: x[:-5], os.listdir("tools/benchmark/inputs")))

print(f"contracts = {contracts}")


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
    assert os.system('cargo run') == 0
    os.chdir('../../')




import pandas as pd


import subprocess



if __name__ == "__main__":

    if os.environ.get("GITHUB_SHA") is None:
        # script running locally
        result = subprocess.run(['bash', '-c', 'git rev-parse --short HEAD'], stdout=subprocess.PIPE)
        commit = result.stdout.decode('utf-8')
        commit = commit[:-1]
    else:
        # script running in github actions
        result = subprocess.run(['bash', '-c', 'git log --pretty=format:\"%h\" -n 2 | tail -1'], stdout=subprocess.PIPE)
        commit = result.stdout.decode('utf-8')
        

    print(f'Commit = {commit}')

    dataframes = []

    for i in range(10):
        clean()
        compile_contracts()
        print("Contracts compiled")
        copy_contracts()
        print("Benchmark started")
        run_bench()
        print("Benchmark ended, see results in tools/benchmark/csvs/{commit}.csv")
        print("Clean started")
        clean()
        print("Clean ended")
        print(os.getcwd())
        dataframes.append(pd.read_csv(f'tools/benchmark/csvs/{commit}.csv'))

    # Extract the 5th column from each DataFrame
    fifth_columns = pd.concat([df.iloc[:, 5] for df in dataframes], axis=1)

    # Calculate the mean, variance, min, and max values for each row in the 5th columns
    mean_5th_column = fifth_columns.mean(axis=1)
    variance_5th_column = fifth_columns.var(axis=1)
    min_5th_column = fifth_columns.min(axis=1)
    max_5th_column = fifth_columns.max(axis=1)

    # Create a new DataFrame using the first DataFrame as a template
    new_df = dataframes[0].copy()

    # Replace the 5th column in the new DataFrame with the mean values
    new_df.iloc[:, 5] = mean_5th_column

    # Add columns for variance, min, and max values
    new_df['Variance'] = variance_5th_column
    new_df['Min'] = min_5th_column
    new_df['Max'] = max_5th_column

    last_value_5th_column = new_df.iloc[-1, 5]   # mean with {"loop_limit": 3000}


    # I runned this code 5 times and see next bounds: [249.8; 255.6]
    UPPER_BOUND = 257
    LOWER_BOUND = 248

    assert last_value_5th_column <= UPPER_BOUND
    assert last_value_5th_column >= LOWER_BOUND

    # Save the new DataFrame to a CSV file
    new_df.to_csv(f"tools/benchmark/csvs/{commit}.csv", index=False)
