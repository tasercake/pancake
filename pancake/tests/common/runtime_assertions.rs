/// Guard a runtime contract with the diagnostic that explains its failure.
///
/// # Errors
///
/// Returns the diagnostic when the runtime violates the contract.
fn require(condition: bool, diagnostic: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| diagnostic.to_owned())
}
