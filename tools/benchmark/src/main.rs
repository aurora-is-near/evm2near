use csv::Writer;
use serde::{Deserialize};
use serde_json::{json, Value};
use std::{fs::File, ffi::OsString};


#[derive(Debug, Deserialize)]
struct Input {
    pub method: Box<str>,
    pub input: Value,
}

const TERA: u64 = 1000000000000_u64;

async fn bench_contract(wtr: &mut Writer<File>, name_os: OsString) -> anyhow::Result<()> {
    let name = &name_os.to_str().unwrap()[0..name_os.len() - 5];
    println!("Name = {}", name);
    let worker = near_workspaces::sandbox().await?;
    let wasm = std::fs::read(format!("{}.wasm", name))?;
    let contract = worker.dev_deploy(&wasm).await?;

    let inputs: Vec<Input> = serde_json::from_str(
        &std::fs::read_to_string(format!("inputs/{}.json", name))
            .expect("Unable to read file"),
    )
    .expect("JSON does not have correct format.");
    let deposit = 10000000000000000000000_u128;
    for input in &inputs {
        let outcome = contract
            .call(&input.method)
            .args_json(json!(input.input))
            .deposit(deposit)
            .gas(near_units::parse_gas!("300000000000000") as u64)
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
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // let contracts = vec!["calc"];

    let paths = std::fs::read_dir("inputs/").unwrap();

    let contracts = paths.into_iter().map(|dir| dir.unwrap().file_name()).collect::<Vec<_>>();

    // let contracts = contracts_os.into_iter().map(|os|os.clone().to_str().unwrap()).collect::<Vec<_>>();


    let mut wtr = Writer::from_path("benchmark.csv")?;
    wtr.write_record([
        "Contract",
        "Method",
        "Gas burned",
        "Gas used",
        "Tgas burned",
        "Tgas used",
        "Input",
    ])?;

    for contract in contracts {
        bench_contract(&mut wtr, contract).await?;
    }

    wtr.flush()?;
    Ok(())
}
