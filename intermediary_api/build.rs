fn main() -> Result<(), Box<dyn std::error::Error>> {
  tonic_build::configure().build_client(false).compile(&["proto/rinha.proto"], &["proto"])?;
  Ok(())
}