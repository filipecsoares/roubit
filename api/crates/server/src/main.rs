//! Roubit REST API Server

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Roubit REST API Server v{}", roubit_core::VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_server_version_is_accessible() {
        assert!(!roubit_core::VERSION.is_empty());
    }
}
