use std::{collections::HashMap, env, path::Path};

use mibpf_tools::{self, execute};

use internal_representation::{BinaryFileLayout, ExecutionModel, TargetVM};
use mibpf_tools::deploy;
use serde::Deserialize;

use dotenv;

pub struct Environment {
    pub mibpf_root_dir: String,
    pub coap_root_dir: String,
    pub riot_instance_net_if: String,
    pub riot_instance_ip: String,
    pub host_net_if: String,
    pub host_ip: String,
    pub board_name: String,
}

pub fn load_env() -> Environment {
    let path = Path::new(".env");
    let _ = dotenv::from_path(path);

    Environment {
        mibpf_root_dir: dotenv::var("MIBPF_ROOT_DIR").unwrap_or_else(|_| "..".to_string()),
        coap_root_dir: dotenv::var("COAP_ROOT_DIR").unwrap_or_else(|_| "../coaproot".to_string()),
        riot_instance_net_if: dotenv::var("RIOT_INSTANCE_NET_IF")
            .unwrap_or_else(|_| "6".to_string()),
        riot_instance_ip: dotenv::var("RIOT_INSTANCE_IP")
            .unwrap_or_else(|_| "fe80::a0d9:ebff:fed5:986b".to_string()),
        host_net_if: dotenv::var("HOST_NET_IF").unwrap_or_else(|_| "tapbr0".to_string()),
        host_ip: dotenv::var("HOST_IP").unwrap_or_else(|_| "fe80::cc9a:73ff:fe4a:47f6".to_string()),
        board_name: dotenv::var("BOARD_NAME").unwrap_or_else(|_| "native".to_string()),
    }
}

pub async fn test_execution(
    test_program: &str,
    layout: BinaryFileLayout,
    environment: &Environment,
) {
    // We first deploy the program on the tested microcontroller
    let result = deploy_test_script(test_program, layout, environment).await;
    assert!(result.is_ok());

    // Then we request execution and check that the return value is what we
    // expected
    let execution_result = execute_deployed_program(0, layout, environment).await;
    if let Err(string) = &execution_result {
        println!("{}", string);
    }
    assert!(execution_result.is_ok());
    let return_value = execution_result.unwrap();

    let expected_return = extract_expected_return(test_program);
    assert!(return_value == expected_return);
}

pub async fn test_execution_accessing_coap_pkt(
    test_program: &str,
    layout: BinaryFileLayout,
    environment: &Environment,
) {
    // We first deploy the program on the tested microcontroller
    let result = deploy_test_script(test_program, layout, environment).await;
    assert!(result.is_ok());

    // Then we request execution and check that the return value is what we
    // expected
    let execution_result = execute_deployed_program_on_coap(0, layout, environment).await;
    if let Err(string) = &execution_result {
        println!("{}", string);
    }
    assert!(execution_result.is_ok());
    let response = execution_result.unwrap();

    let expected = extract_expected_response(test_program);
    assert!(response == expected);
}

const TEST_SOURCES_DIR: &'static str = "tests/test-sources";

/// Test utility funciton used for sending the eBPF scripts to the device given
/// the environment configuration.
pub async fn deploy_test_script(
    file_name: &str,
    layout: BinaryFileLayout,
    environment: &Environment,
) -> Result<(), String> {
    let file_path = format!("{}/{}", TEST_SOURCES_DIR, file_name);
    let out_dir = format!("{}/out", TEST_SOURCES_DIR);
    deploy(
        &file_path,
        &out_dir,
        layout,
        &environment.coap_root_dir,
        0,
        &environment.riot_instance_net_if,
        &environment.riot_instance_ip,
        &environment.host_net_if,
        &environment.host_ip,
        &environment.board_name,
        Some(&environment.mibpf_root_dir),
    )
    .await
}

/// Reads the annotation present at the top of test source files that specifies
/// what the expected response from the program executing with access to the CoAP
/// network packet should be.
pub fn extract_expected_response(file_name: &str) -> String {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let file_path = format!("{}/{}", TEST_SOURCES_DIR, file_name);
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);
    let first_line = reader.lines().next().unwrap().unwrap();
    // The format of the first line is: // TEST_RESULT: {response}
    let mut first_line_iter = first_line.split(" ");

    // We skip the first two tokens: '//' and 'TEST_RESULT' and then collect the
    // rest in case the response contains spaces
    first_line_iter.next();
    first_line_iter.next();

    let response = first_line_iter.collect::<Vec<&str>>().join(" ");
    response
}

/// Reads the annotation present at the top of test source files that specifies
/// what the expected return value of the program should be.
pub fn extract_expected_return(file_name: &str) -> i32 {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let file_path = format!("{}/{}", TEST_SOURCES_DIR, file_name);
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);
    let first_line = reader.lines().next().unwrap().unwrap();
    // The format of the first line is: // TEST_RESULT: 0
    first_line
        .split(" ")
        .nth(2)
        .unwrap()
        .parse::<i32>()
        .unwrap()
}

/// Sends a request to the server to start executing the program located in
/// the specified storage slot using the functionality of executing eBPF programs
/// that have access to the incoming packet context. The response should be
/// written into the packet buffer by the eBPF program and is returned from
/// this function once we receive it.
pub async fn execute_deployed_program_on_coap(
    suit_storage_slot: usize,
    layout: BinaryFileLayout,
    environment: &Environment,
) -> Result<String, String> {
    let available_helpers = (0..23).into_iter().collect::<Vec<u8>>();
    let response = execute(
        &environment.riot_instance_ip,
        TargetVM::Rbpf,
        layout,
        suit_storage_slot,
        &environment.host_net_if,
        ExecutionModel::WithAccessToCoapPacket,
        &available_helpers,
    )
    .await?;

    println!("Response: {}", response);
    // we need to remove the null terminator that we get in the response
    let response = response.trim_matches(char::from(0));
    Ok(response.to_string())
}

pub async fn execute_deployed_program(
    suit_storage_slot: usize,
    layout: BinaryFileLayout,
    environment: &Environment,
) -> Result<i32, String> {
    let available_helpers = (0..23).into_iter().collect::<Vec<u8>>();
    let response = execute(
        &environment.riot_instance_ip,
        TargetVM::Rbpf,
        layout,
        suit_storage_slot,
        &environment.host_net_if,
        ExecutionModel::ShortLived,
        &available_helpers,
    )
    .await?;

    // Short lived executions always return responses of this form:
    // {"execution_time": 10, "result": 0}
    #[derive(Deserialize)]
    struct Response {
        // Execution time in milliseconds
        execution_time: u32,
        // Return value of the program
        result: i32,
    }

    println!("Response: {}", response);
    let response = serde_json::from_str::<Response>(&response)
        .map_err(|e| format!("Failed to parse the json response: {}", e))?;

    Ok(response.result)
}