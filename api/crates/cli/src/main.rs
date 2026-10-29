//! Roubit Command Line Interface

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Roubit CLI v{}", roubit_core::VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cli_sanity() {
        assert!(true);
    }
}
