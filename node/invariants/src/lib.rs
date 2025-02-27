mod invariant_result;
pub use invariant_result::{InvariantIgnoreReason, InvariantResult};

pub mod no_recursion;
use no_recursion::*;

pub mod p2p;
use p2p::*;

pub mod transition_frontier;
use transition_frontier::*;

pub use node::core::invariants::{InvariantService, InvariantsState};

use strum_macros::{EnumDiscriminants, EnumIter, EnumString, IntoStaticStr};

use node::{ActionKind, ActionWithMeta, Service, Store};

pub trait Invariant {
    /// Internal state of the invariant.
    ///
    /// If some state needs to be preserved across checks,
    /// this is the place.
    type InternalState: 'static + Send + Default;

    /// Whether or not invariant is cluster-wide, or for just local node.
    fn is_global(&self) -> bool {
        false
    }

    /// Invariant triggers define a list actions, which should cause
    /// `Invariant::check` to be called.
    ///
    /// If empty, an invariant will never be checked!
    fn triggers(&self) -> &[ActionKind];

    /// Checks the state for invariant violation.
    fn check<S: Service>(
        self,
        internal_state: &mut Self::InternalState,
        store: &Store<S>,
        action: &ActionWithMeta,
    ) -> InvariantResult;
}

macro_rules! define_invariants_enum {
    ($($invariant: ident,)+) => {
        #[derive(EnumIter, EnumString, IntoStaticStr, EnumDiscriminants, Clone, Copy)]
        #[strum(serialize_all = "snake_case")]
        pub enum Invariants {
            $($invariant($invariant),)*
        }

        impl Invariants {
            pub fn index(self) -> usize {
                InvariantsDiscriminants::from(self) as usize
            }

            pub fn is_global(&self) -> bool {
                match self {
                    $(Self::$invariant(invariant) => invariant.is_global(),)*
                }
            }

            pub fn triggers(&self) -> &[ActionKind] {
                match self {
                    $(Self::$invariant(invariant) => invariant.triggers(),)*
                }
            }

            pub fn check<S: Service + InvariantService>(
                self,
                store: &mut Store<S>,
                action: &ActionWithMeta,
            ) -> InvariantResult {
                let mut invariants_state = if self.is_global() {
                    match store.service.cluster_invariants_state() {
                        Some(mut v) => v.take(),
                        None => return InvariantResult::Ignored(InvariantIgnoreReason::GlobalInvariantNotInTestingCluster),
                    }
                } else {
                    store.service.invariants_state().take()
                };

                let res = match self {
                    $(Self::$invariant(invariant) => {
                        let invariant_state = invariants_state.get(self.index());
                        invariant.check(invariant_state, store, action)
                    })*
                };

                if self.is_global() {
                    match store.service.cluster_invariants_state() {
                        Some(mut s) =>
                        *s = invariants_state,
                        None => unreachable!("function should have returned above"),
                    }
                } else {
                    *store.service.invariants_state() = invariants_state;
                };

                res
            }
        }
    };
}

define_invariants_enum! {
    NoRecursion,
    P2pStatesAreConsistent,
    TransitionFrontierOnlySyncsToBetterBlocks,
}

lazy_static::lazy_static! {
    /// List of invariants that need to be triggered if we see a given `ActionKind`.
    static ref INVARIANTS_BY_ACTION_KIND: Vec<Vec<Invariants>> = {
        let mut by_action_kind = Vec::new();
        by_action_kind.resize_with(ActionKind::COUNT as usize, Vec::new);
        for invariant in Invariants::iter() {
            for action_kind in invariant.triggers() {
                let v = by_action_kind.get_mut(*action_kind as usize).unwrap();
                v.push(invariant);
            }
        }
        by_action_kind
    };
}

impl Invariants {
    pub fn iter() -> impl Iterator<Item = Invariants> {
        <Self as strum::IntoEnumIterator>::iter()
    }

    pub fn check_all<'a, S: Service + InvariantService>(
        store: &'a mut Store<S>,
        action: &'a ActionWithMeta,
    ) -> impl 'a + Iterator<Item = (Self, InvariantResult)> {
        let action_kind = action.action().kind();
        INVARIANTS_BY_ACTION_KIND
            .get(action_kind as usize)
            .unwrap()
            .iter()
            .map(|invariant| (*invariant, invariant.check(store, action)))
    }

    pub fn to_str(self) -> &'static str {
        self.into()
    }
}
