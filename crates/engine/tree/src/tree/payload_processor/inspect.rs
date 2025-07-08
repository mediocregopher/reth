//! Utilities for inspecting the input/output of payload processing

use crate::tree::{
    multiproof::SparseTrieUpdate, sparse_trie::update_sparse_trie, StateProviderBuilder,
    StateProviderDatabase,
};
use alloy_consensus::BlockHeader;
use alloy_primitives::B256;
use reth_evm::{
    execute::{BasicBlockExecutor, Executor},
    ConfigureEvm,
};
use reth_primitives_traits::{NodePrimitives, RecoveredBlock};
use reth_provider::{
    BlockReader, DatabaseProviderFactory, StateCommitmentProvider, StateProviderFactory,
    StateReader,
};
use reth_trie::{DecodedMultiProof, HashedPostState, TrieInput};
use reth_trie_common::updates::TrieUpdates;
use reth_trie_sparse::{
    provider::DefaultTrieNodeProviderFactory, SerialSparseTrie, SparseStateTrie,
};

/// TODO
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SparseTrieInputs {
    /// TODO
    pub hashed_state: HashedPostState,
    /// TODO
    pub decoded_multiproof: DecodedMultiProof,
}

/// TODO
#[derive(Debug)]
pub struct PayloadInspector<N, Evm>
where
    N: NodePrimitives,
    Evm: ConfigureEvm<Primitives = N>,
{
    evm_config: Evm,
}

/// TODO
impl<N, Evm> PayloadInspector<N, Evm>
where
    N: NodePrimitives,
    Evm: ConfigureEvm<Primitives = N> + 'static,
{
    /// TODO
    pub const fn new(evm_config: Evm) -> Self {
        Self { evm_config }
    }

    /// TODO
    pub fn sparse_trie_inputs<P>(
        &self,
        factory: P,
        recovered_block: &RecoveredBlock<N::Block>,
    ) -> eyre::Result<SparseTrieInputs>
    where
        P: DatabaseProviderFactory<Provider: BlockReader>
            + BlockReader
            + StateProviderFactory
            + StateReader
            + StateCommitmentProvider
            + Clone
            + 'static,
    {
        // Prepare a StateProviderDatabase for use with the EVM
        let parent_hash = recovered_block.parent_hash();
        let state_provider_builder =
            StateProviderBuilder::<N, _>::new(factory, parent_hash, None::<Vec<_>>);
        let db = StateProviderDatabase::new(state_provider_builder.build()?);

        // Initialize EVM config and block executor
        let executor = BasicBlockExecutor::new(self.evm_config.clone(), db);

        // Execute the entire block
        let bundle_state = executor
            .execute(recovered_block)
            .map_err(|e| eyre::eyre!("Failed to execute block: {}", e))?
            .state;

        // Convert bundle state to HashedPostState
        let hashed_state =
            HashedPostState::from_bundle_state::<reth_trie::KeccakKeyHasher>(bundle_state.state());

        // Create TrieInput and MultiProofTargets from HashedPostState
        let trie_input = TrieInput::from_state(hashed_state.clone());
        let proof_targets = hashed_state.multi_proof_targets();

        // Create calculate DecodedMultiProof from DB state
        let multiproof = state_provider_builder.build()?.multiproof(trie_input, proof_targets)?;
        let decoded_multiproof: DecodedMultiProof =
            multiproof.try_into().map_err(|e| eyre::eyre!("Failed to decode multiproof: {}", e))?;

        Ok(SparseTrieInputs { hashed_state, decoded_multiproof })
    }
}

/// TODO
pub fn sparse_trie_root_and_updates(inputs: SparseTrieInputs) -> eyre::Result<(B256, TrieUpdates)> {
    let SparseTrieInputs { hashed_state, decoded_multiproof, .. } = inputs;

    // TODO something needs to be done about this
    let trie_node_provider_factory = DefaultTrieNodeProviderFactory;

    let mut sparse_trie: SparseStateTrie<SerialSparseTrie, SerialSparseTrie> =
        SparseStateTrie::new().with_updates(true);

    update_sparse_trie(
        &mut sparse_trie,
        SparseTrieUpdate { state: hashed_state, multiproof: decoded_multiproof },
        &trie_node_provider_factory,
    )
    .map_err(|e| eyre::eyre!("updating sparse trie: {e:?}"))?;

    sparse_trie
        .root_with_updates(trie_node_provider_factory)
        .map_err(|e| eyre::eyre!("calculating state root and updates: {e:?}"))
}
