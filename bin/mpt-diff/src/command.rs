//! Command implementation for mpt-diff

use clap::Parser;
use eyre::{OptionExt, Result};
use reth_cli::chainspec::ChainSpecParser;
use reth_cli_runner::CliContext;
use reth_engine_tree::tree::inspect::PayloadInspector;
use reth_ethereum::{
    node::{EthEvmConfig, EthereumNode},
    primitives::AlloyBlockHeader,
    provider::{providers::ReadOnlyConfig, BlockReader},
};
use reth_ethereum_cli::chainspec::EthereumChainSpecParser;
use reth_node_core::args::{DatadirArgs, LogArgs};
use reth_provider::providers::BlockchainProvider;
use reth_tracing::FileWorkerGuard;
use std::sync::Arc;

/// Generate diff files for MPT state transitions
#[derive(Debug, Parser)]
pub(crate) struct MptDiffCommand {
    /// Block number to generate diff from
    #[arg(long)]
    pub block: u64,

    /// Parameters for datadir configuration
    #[command(flatten)]
    pub datadir: DatadirArgs,

    /// The chain this node is running.
    ///
    /// Possible values are either a built-in chain or the path to a chain specification file.
    #[arg(
        long,
        value_name = "CHAIN_OR_PATH",
        long_help = EthereumChainSpecParser::help_message(),
        default_value = EthereumChainSpecParser::SUPPORTED_CHAINS[0],
        value_parser = EthereumChainSpecParser::parser(),
        global = true
    )]
    pub chain: Arc<<EthereumChainSpecParser as ChainSpecParser>::ChainSpec>,

    #[command(flatten)]
    logs: LogArgs,
}

impl MptDiffCommand {
    /// Execute the mpt-diff command
    pub(crate) async fn execute(self, _ctx: CliContext) -> eyre::Result<()> {
        // Initialize tracing
        let _guard = self.init_tracing()?;

        let data_dir = self.datadir.clone().resolve_datadir(self.chain.chain());
        let factory = EthereumNode::provider_factory_builder()
            .open_read_only(self.chain.clone(), ReadOnlyConfig::from_datadir(data_dir.clone()))?;
        let block_number = self.block;

        tracing::debug!(
            "Generating MPT diff for block {} using datadir {}",
            self.block,
            data_dir.data_dir().display(),
        );

        // Open database and fetch the recovered block
        let provider = factory.provider()?;
        let recovered_block = provider
            .recovered_block(block_number.into(), reth_provider::TransactionVariant::NoHash)?
            .ok_or_eyre("block not found")?;

        let block_state_root = recovered_block.sealed_block().header().state_root;
        tracing::debug!("Block header state root: {:#x}", block_state_root);

        // Create inspector and calculate the HashedPostState and DecodedMultiProof
        let evm_config = EthEvmConfig::ethereum(self.chain);
        let blockchain_provider = BlockchainProvider::new(factory)?;
        let inspector = PayloadInspector::new(evm_config);
        let sparse_trie_inputs =
            inspector.sparse_trie_inputs(blockchain_provider, &recovered_block)?;

        // Output detailed JSON structure with the collected data
        let diff_json = serde_json::json!({
            "trie_inputs": sparse_trie_inputs,
            "block": {
                "number": self.block,
                "hash": recovered_block.hash(),
                "state_root": recovered_block.state_root(),
            },
        });

        println!("{}", serde_json::to_string_pretty(&diff_json)?);

        tracing::debug!("MPT diff generation completed");

        Ok(())
    }

    /// Initializes tracing with the configured options.
    ///
    /// If file logging is enabled, this function returns a guard that must be kept alive to ensure
    /// that all logs are flushed to disk.
    pub(crate) fn init_tracing(&self) -> Result<Option<FileWorkerGuard>> {
        let guard = self.logs.init_tracing()?;
        Ok(guard)
    }
}
