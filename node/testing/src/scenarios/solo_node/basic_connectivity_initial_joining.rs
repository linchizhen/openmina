#![allow(warnings)]

use std::{collections::HashMap, time::Duration};

use libp2p::Multiaddr;

use node::p2p::connection::outgoing::P2pConnectionOutgoingInitOpts;

use crate::{
    hosts,
    node::RustNodeTestingConfig,
    scenario::{ListenerNode, ScenarioStep},
    scenarios::ClusterRunner,
};

/// Local test to ensure that the Openmina node can connect to an existing OCaml testnet.
/// Launch an Openmina node and connect it to seed nodes of the public (or private) OCaml testnet.
/// Run the simulation until:
/// * Number of known peers is greater than or equal to the maximum number of peers.
/// * Number of connected peers is greater than or equal to some threshold.
/// Fail the test if the specified number of steps occur but the condition is not met.
#[derive(documented::Documented, Default, Clone, Copy)]
pub struct SoloNodeBasicConnectivityInitialJoining;

impl SoloNodeBasicConnectivityInitialJoining {
    pub async fn run(self, mut runner: ClusterRunner<'_>) {
        const MAX_PEERS_PER_NODE: usize = 100;
        const KNOWN_PEERS: usize = 5; // current devnet network
        const STEPS: usize = 3_000;
        const STEP_DELAY: Duration = Duration::from_millis(200);

        let initial_peers = hosts::devnet();
        eprintln!("set max peers per node: {MAX_PEERS_PER_NODE}");
        for seed in &initial_peers {
            eprintln!("add initial peer: {seed:?}");
        }
        let config = RustNodeTestingConfig::devnet_default()
            .max_peers(MAX_PEERS_PER_NODE)
            .initial_peers(initial_peers);

        let node_id = runner.add_rust_node(config);
        let peer_id = libp2p::PeerId::try_from(
            runner
                .node(node_id)
                .expect("must exist")
                .state()
                .p2p
                .my_id(),
        )
        .unwrap();
        eprintln!("launch Openmina node, id: {node_id}, peer_id: {peer_id}");

        for step in 0..STEPS {
            tokio::time::sleep(STEP_DELAY).await;

            let steps = runner
                .pending_events(true)
                .map(|(node_id, _, events)| {
                    events.map(move |(_, event)| ScenarioStep::Event {
                        node_id,
                        event: event.to_string(),
                    })
                })
                .flatten()
                .collect::<Vec<_>>();

            for step in steps {
                runner.exec_step(step).await.unwrap();
            }

            runner
                .exec_step(ScenarioStep::AdvanceNodeTime {
                    node_id,
                    by_nanos: STEP_DELAY.as_nanos() as _,
                })
                .await
                .unwrap();

            runner
                .exec_step(ScenarioStep::CheckTimeouts { node_id })
                .await
                .unwrap();

            let node = runner.node(node_id).expect("must exist");
            let ready_peers = node.state().p2p.ready_peers_iter().count();
            let my_id = node.state().p2p.my_id();
            let known_peers: usize = node
                .state()
                .p2p
                .ready()
                .and_then(|p2p| p2p.network.scheduler.discovery_state())
                .map_or(0, |discovery_state| {
                    discovery_state
                        .routing_table
                        .closest_peers(&my_id.try_into().unwrap())
                        .count()
                });

            println!("step: {step}");
            println!("known peers: {known_peers}");
            println!("connected peers: {ready_peers}");

            // TODO: the threshold is too small, node cannot connect to many peer before the timeout
            if ready_peers >= KNOWN_PEERS && known_peers >= KNOWN_PEERS {
                eprintln!("step: {step}");
                eprintln!("known peers: {known_peers}");
                eprintln!("connected peers: {ready_peers}");
                eprintln!("success");

                if let Some(debugger) = runner.debugger() {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    let connections = debugger
                        .connections_raw(0)
                        .map(|(id, c)| (id, (c.info.addr, c.info.fd, c.info.pid, c.incoming)))
                        .collect::<HashMap<_, _>>();

                    // dbg
                    for (id, cn) in &connections {
                        eprintln!("{id}: {}", serde_json::to_string(cn).unwrap());
                    }
                    // dbg
                    for (id, msg) in debugger.messages(0, "") {
                        eprintln!("{id}: {}", serde_json::to_string(&msg).unwrap());
                    }
                    // TODO: fix debugger returns timeout
                    let connections = debugger
                        .connections()
                        .filter_map(|id| Some((id, connections.get(&id)?.clone())))
                        .collect::<HashMap<_, _>>();
                    let incoming = connections.iter().filter(|(_, (_, _, _, i))| *i).count();
                    let outgoing = connections.len() - incoming;
                    eprintln!(
                        "debugger seen {incoming} incoming connections and {outgoing} outgoing connections",
                    );
                    let state_machine_peers = if cfg!(feature = "p2p-webrtc") {
                        ready_peers
                    } else {
                        ready_peers.max(known_peers)
                    };
                    assert_eq!(
                        incoming + outgoing,
                        state_machine_peers,
                        "debugger must see the same number of connections as the state machine"
                    );
                } else {
                    eprintln!("no debugger, run test with --use-debugger for additional check");
                }

                return;
            }
        }

        panic!("timeout");
    }
}
