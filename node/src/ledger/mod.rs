pub mod read;
pub mod write;

mod ledger_config;
use ark_ff::fields::arithmetic::InvalidBigInt;
pub use ledger_config::*;

mod ledger_event;
pub use ledger_event::*;

mod ledger_actions;
pub use ledger_actions::*;

mod ledger_state;
pub use ledger_state::*;

mod ledger_reducer;

mod ledger_service;
pub use ledger_service::*;

pub mod ledger_manager;

pub use ledger::AccountIndex as LedgerAccountIndex;
pub use ledger::Address as LedgerAddress;
pub use ledger_manager::LedgerManager;

use ledger::TreeVersion;
use mina_p2p_messages::v2;

// FIXME(tizoc): both networks use the same value, but this will break if that changes
pub const LEDGER_DEPTH: usize =
    crate::core::network::mainnet::CONSTRAINT_CONSTANTS.ledger_depth as usize;

lazy_static::lazy_static! {
    /// Array size needs to be changed when the tree's depth change
    static ref LEDGER_HASH_EMPTIES: [v2::LedgerHash; LEDGER_DEPTH + 1] = {
        use ledger::TreeVersion;

        std::array::from_fn(|i| {
            let hash = ledger::V2::empty_hash_at_height(LEDGER_DEPTH.saturating_sub(i));
            v2::MinaBaseLedgerHash0StableV1(hash.into()).into()
        })
    };
}

pub fn ledger_empty_hash_at_depth(depth: usize) -> v2::LedgerHash {
    LEDGER_HASH_EMPTIES.get(depth).unwrap().clone()
}

/// Given the hash of the subtree containing all accounts of height `subtree_height`
/// compute the hash of a tree of size `LEDGER_DEPTH` if all other nodes were
/// empty.
pub fn complete_height_tree_with_empties(
    content_hash: &v2::LedgerHash,
    subtree_height: usize,
) -> Result<v2::LedgerHash, InvalidBigInt> {
    assert!(LEDGER_DEPTH >= subtree_height);
    let content_hash = content_hash.0.to_field()?;

    let computed_hash = (subtree_height..LEDGER_DEPTH).fold(content_hash, |prev_hash, height| {
        let depth = LEDGER_DEPTH.saturating_sub(height);
        let empty_right = ledger_empty_hash_at_depth(depth).0.to_field().unwrap(); // We know empties are valid
        ledger::V2::hash_node(height, prev_hash, empty_right)
    });

    Ok(v2::LedgerHash::from_fp(computed_hash))
}

/// Returns the minimum tree height required for storing `num_accounts` accounts.
pub fn tree_height_for_num_accounts(num_accounts: u64) -> usize {
    if num_accounts == 1 {
        1
    } else if num_accounts.is_power_of_two() {
        num_accounts.ilog2() as usize
    } else {
        num_accounts.next_power_of_two().ilog2() as usize
    }
}

/// Given the hash of the subtree containing `num_accounts` accounts
/// compute the hash of a tree of size `LEDGER_DEPTH` if all other nodes were
/// empty.
///
/// NOTE: For out of range sizes, en empty tree hash is returned.
pub fn complete_num_accounts_tree_with_empties(
    contents_hash: &v2::LedgerHash,
    num_accounts: u64,
) -> Result<v2::LedgerHash, InvalidBigInt> {
    // Note, we assume there is always at least one account
    if num_accounts == 0 {
        return Ok(ledger_empty_hash_at_depth(0));
    }

    let subtree_height = tree_height_for_num_accounts(num_accounts);

    // This would not be a valid number of accounts because it doesn't fit the tree
    if subtree_height > LEDGER_DEPTH {
        Ok(ledger_empty_hash_at_depth(0))
    } else {
        complete_height_tree_with_empties(contents_hash, subtree_height)
    }
}

pub fn hash_node_at_depth(
    depth: usize,
    left: mina_hasher::Fp,
    right: mina_hasher::Fp,
) -> mina_hasher::Fp {
    let height = LEDGER_DEPTH.saturating_sub(depth).saturating_sub(1);
    ledger::V2::hash_node(height, left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_with_empties() {
        let subtree_height = 14;
        let expected_hash: v2::LedgerHash = "jwxdRe86RJV99CZbxZzb4JoDwEnvNQbc6Ha8iPx7pr3FxYpjHBG"
            .parse()
            .unwrap();
        let contents_hash = "jwav4pBszibQqek634VUQEc5WZAbF3CnT7sMyhqXe3vucyXdjJs"
            .parse()
            .unwrap();

        let actual_hash =
            complete_height_tree_with_empties(&contents_hash, subtree_height).unwrap();

        assert_eq!(expected_hash, actual_hash);
    }

    #[test]
    fn test_complete_with_empties_with_num_accounts() {
        let subtree_height = 8517;
        let expected_hash: v2::LedgerHash = "jwxdRe86RJV99CZbxZzb4JoDwEnvNQbc6Ha8iPx7pr3FxYpjHBG"
            .parse()
            .unwrap();
        let contents_hash = "jwav4pBszibQqek634VUQEc5WZAbF3CnT7sMyhqXe3vucyXdjJs"
            .parse()
            .unwrap();

        let actual_hash =
            complete_num_accounts_tree_with_empties(&contents_hash, subtree_height).unwrap();

        assert_eq!(expected_hash, actual_hash);
    }
}
