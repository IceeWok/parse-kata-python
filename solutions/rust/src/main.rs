use async_std::fs::File;
use async_std::io::{BufReader, BufWriter};
use async_std::prelude::*;
use futures::stream::{futures_unordered::FuturesUnordered, StreamExt};
use serde_json::Value;
use std::env;
use std::error::Error;

async fn process_input_file(input_path: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
    let input_file = File::open(input_path).await?;
    let output_file = File::create(output_path).await?;
    let input_buffered_reader = BufReader::new(input_file);
    let mut output_buffered_writer = BufWriter::new(output_file);

    let mut line_stream = input_buffered_reader.lines();
    let mut futures = FuturesUnordered::new();

    while let Some(line_result) = line_stream.next().await {
        let line = line_result?;
        futures.push(async_std::task::spawn_blocking(move || {
            let fields: Vec<&str> = line.splitn(5, '\t').collect();
            if fields.len() == 5 {
                let json_value: Value =
                    serde_json::from_str(fields[4]).map_err(|e| e.to_string())?;
                if let Some(title) = json_value.get("title") {
                    if let Some(title_str) = title.as_str() {
                        return Ok(Some(title_str.to_string()));
                    }
                }
            }
            Ok::<Option<String>, String>(None)
        }));
    }

    while let Some(result) = futures.next().await {
        let title_opt = result.map_err(|e| format!("Error processing line: {}", e));
        if let Ok(Some(title)) = title_opt {
            output_buffered_writer
                .write_all(format!("{}\n", title).as_bytes())
                .await?;
        }
    }

    // Flush the writer to ensure all output is written to the file
    output_buffered_writer.flush().await?;

    Ok(())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} <input_file> <output_file>", args[0]);
        return Ok(());
    }

    let input_path = &args[1];
    let output_path = &args[2];

    process_input_file(input_path, output_path)
        .await
        .map_err(|err| println!("Error processing file: {}", err))
        .ok();

    println!("Processing complete.");

    Ok(())
}
