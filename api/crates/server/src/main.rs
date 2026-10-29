//! Roubit REST API Server

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Roubit REST API Server v{}", roubit_core::VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_server_sanity() {
        assert!(true);
    }
}
