fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("src/proto/narrative.proto")?;
    tonic_build::compile_protos("src/proto/soul.proto")?;
    Ok(())
}