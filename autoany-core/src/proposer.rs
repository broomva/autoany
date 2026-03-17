use crate::error::Result;
use crate::ledger::Ledger;
use crate::types::Mutation;

/// Proposes mutations to the current artifact state.
///
/// This is where LLM reasoning is most valuable — generating
/// semantically meaningful mutations rather than blind search.
pub trait Proposer {
    /// The artifact type to mutate.
    type Artifact;

    /// Propose a mutation given the current artifact and search history.
    fn propose(
        &self,
        artifact: &Self::Artifact,
        ledger: &Ledger,
    ) -> Result<(Mutation, Self::Artifact)>;
}
