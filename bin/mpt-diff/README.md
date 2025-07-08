# mpt-diff

The mpt-diff tool generates a diff file containing all data required to
compute a new root hash after a state transition is performed. This data
includes:

* Proofs for all affected intermediate nodes and leaves in the starting MPT
* Leaf update/removals
* The expected root hash after all leaf update/removals are applied.

The diff itself is simply a JSON file. Once generated the file can be used to
inspect changes to account and storage roots, as well as be used for testing MPT
implementations.

## Usage

mpt-diff requires the data directory synced to the block from which the diff
will be generated. It also requires an RPC endpoint to fetch the block from. It
does not require a running reth node.

```bash
mpt-tool --rpc-url <RPC_URL> --datadir <DATA_DIR> --block <N> output.json
```
