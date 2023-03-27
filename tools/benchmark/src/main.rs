use csv::Writer;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::process::Command;
use std::{ffi::OsString, fs::File};

#[derive(Debug, Deserialize)]
struct Input {
    pub method: Box<str>,
    pub input: Value,
}

const TERA: u64 = 1000000000000_u64;

const ASSERTATION_LOWER_BOUND: u64 = 230_u64;
const ASSERTATION_UPPER_BOUND: u64 = 280_u64;

async fn assert_bench() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let wasm = std::fs::read("bench.wasm")?;
    let contract = worker.dev_deploy(&wasm).await?;

    let deposit = 10000000000000000000000_u128;

    let outcome = contract
        .call("cpu_ram_soak_test")
        .args_json(json!({"loop_limit": 3000}))
        .deposit(deposit)
        .gas(near_units::parse_gas!("300 TGas") as u64)
        .transact()
        .await?;
    for failure in &outcome.failures() {
        println!("{:#?}", failure);
    }
    assert!(outcome.is_success());

    let gas_used = outcome.total_gas_burnt / TERA;

    println!("GAS USED = {}", gas_used);

    assert!(gas_used >= ASSERTATION_LOWER_BOUND);
    assert!(gas_used <= ASSERTATION_UPPER_BOUND);

    Ok(())
}

async fn bench_contract(
    wtr: &mut Writer<File>,
    name_os: OsString,
    commit: String,
) -> anyhow::Result<()> {
    let name = &name_os.to_str().unwrap()[0..name_os.len() - 5];
    println!("Name = {}", name);
    let worker = near_workspaces::sandbox().await?;
    let wasm = std::fs::read(format!("{}.wasm", name))?;
    let contract = worker.dev_deploy(&wasm).await?;

    let inputs: Vec<Input> = serde_json::from_str(
        &std::fs::read_to_string(format!("inputs/{}.json", name)).expect("Unable to read file"),
    )
    .expect("JSON does not have correct format.");
    let deposit = 10000000000000000000000_u128;
    for input in &inputs {
        let outcome = contract
            .call(&input.method)
            .args_json(json!(input.input))
            .deposit(deposit)
            .gas(near_units::parse_gas!("300 TGas") as u64)
            .transact()
            .await?;
        for failure in &outcome.failures() {
            println!("{:#?}", failure);
        }
        assert!(outcome.is_success());

        wtr.write_record(&[
            name.to_string(),
            input.method.to_string(),
            outcome.outcome().gas_burnt.to_string(),
            outcome.total_gas_burnt.to_string(),
            (outcome.outcome().gas_burnt / TERA).to_string(),
            (outcome.total_gas_burnt / TERA).to_string(),
            input.input.to_string(),
            commit.clone(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    assert_bench().await?;

    let paths = std::fs::read_dir("inputs/").unwrap();

    let contracts = paths
        .into_iter()
        .map(|dir| dir.unwrap().file_name())
        .collect::<Vec<_>>();

    // let github_sha = env::var("GITHUB_SHA").expect("GITHUB_SHA environment variable not found");
    // println!("GITHUB_SHA: {}", github_sha);

    let commit = match env::var("GITHUB_SHA") {
        Ok(_) => {
            println!("ENVVAR exist");
            let output = Command::new("sh")
                .arg("-c")
                .arg("git log --pretty=format:\"%h\" -n 2 | tail -1")
                .output()
                .expect("failed to execute process");

            let stdout = output.stdout;
            let tmp = std::str::from_utf8(&stdout).unwrap().to_string();
            tmp
        }
        Err(_) => {
            println!("ENVVAR don't exist");
            let output = Command::new("sh")
                .arg("-c")
                .arg("git rev-parse --short HEAD")
                .output()
                .expect("failed to execute process");

            let stdout = output.stdout;
            let mut tmp = std::str::from_utf8(&stdout).unwrap().to_string();
            tmp.pop(); // to remove \n in the end
            tmp
        }
    };
    println!("Commit = {}", commit);

    let mut wtr = Writer::from_path(format!("csvs/{}.csv", commit))?;

    wtr.write_record([
        "Contract",
        "Method",
        "Gas burned",
        "Gas used",
        "Tgas burned",
        "Tgas used",
        "Input",
        "Commit",
    ])?;

    for contract in contracts {
        bench_contract(&mut wtr, contract, commit.clone()).await?;
    }

    wtr.flush()?;
    Ok(())
}
