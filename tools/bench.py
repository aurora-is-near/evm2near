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

    for i in range(20):
        clean()
        compile_contracts()
        print("Contracts compiled")
        copy_contracts()
        print("Benchmark started")
        run_bench()
        print(f"Benchmark ended, see results in tools/benchmark/csvs/{commit}.csv")
        print("Clean started")
        clean()
        print("Clean ended")
        print(os.getcwd())
        dataframes.append(pd.read_csv(f'tools/benchmark/csvs/{commit}.csv'))

    # Extract the 5th column from each DataFrame
    Tgas_used = pd.concat([df.iloc[:, 5] for df in dataframes], axis=1)

    # Calculate the mean, variance, min, and max values for each row in the 5th columns
    mean_Tgas_used = Tgas_used.mean(axis=1)
    variance_Tgas_used = Tgas_used.var(axis=1)
    min_Tgas_used = Tgas_used.min(axis=1)
    max_Tgas_used = Tgas_used.max(axis=1)

    # Create a new DataFrame using the first DataFrame as a template
    new_df = dataframes[0].copy()

    # Replace the 5th column in the new DataFrame with the mean values
    new_df.iloc[:, 5] = mean_Tgas_used

    # Add columns for variance, min, and max values
    new_df['Variance'] = variance_Tgas_used
    new_df['Min'] = min_Tgas_used
    new_df['Max'] = max_Tgas_used

    # Save the new DataFrame to a CSV file
    new_df.to_csv(f"tools/benchmark/csvs/{commit}.csv", index=False)

    # extract mean and variance for bench with loop_limit = 3000
    mean = new_df.iloc[-1, 5]   
    variance = new_df.iloc[-1, 6]

    print(f"Mean = {mean}\nVariance = {variance}")
   
    UPPER_BOUND_MEAN = 255
    LOWER_BOUND_MEAN = 245

    # I runned code three times and values was 109.6, 77.7, 49.3, so this bounds will be changed soon, I think
    UPPER_BOUND_VARIANCE = 130
    LOWER_BOUND_VARIANCE = 40

    assert mean <= UPPER_BOUND_MEAN
    assert mean >= LOWER_BOUND_MEAN

    assert variance <= UPPER_BOUND_VARIANCE
    assert variance >= LOWER_BOUND_VARIANCE


