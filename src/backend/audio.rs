use crate::events::EventBus;
use anyhow::{Context, Result};
use pipewire as pw;
use pw::node::{Node, NodeInfoRef, NodeState};
use pw::proxy::ProxyT;
use pw::registry::GlobalObject;
use pw::types::ObjectType;
use spa::utils::dict::DictRef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::thread;
use tokio::sync::watch;
use tracing::{debug, warn};

pub fn spawn_audio_monitor(event_bus: EventBus, mut shutdown: watch::Receiver<bool>) {
    let (sender, receiver) = pw::channel::channel::<Control>();
    tokio::spawn(async move {
        let _ = shutdown.changed().await;
        let _ = sender.send(Control::Quit);
    });

    thread::spawn(move || {
        if let Err(error) = run_audio_monitor(event_bus, receiver) {
            warn!("audio monitor failed: {error:#}");
        }
    });
}

enum Control {
    Quit,
}

struct BoundNode {
    _node: Node,
    _listener: pw::node::NodeListener,
    _proxy_listener: pw::proxy::ProxyListener,
}

#[derive(Clone, Debug)]
struct AudioNodeSnapshot {
    id: u32,
    name: String,
    state: String,
    volume: f64,
    muted: bool,
}

fn run_audio_monitor(event_bus: EventBus, receiver: pw::channel::Receiver<Control>) -> Result<()> {
    pw::init();

    let mainloop = pw::main_loop::MainLoop::new(None).context("creating PipeWire main loop")?;
    let _control = receiver.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        move |message| match message {
            Control::Quit => mainloop.quit(),
        }
    });

    let context = pw::context::Context::new(&mainloop).context("creating PipeWire context")?;
    let core = context
        .connect(None)
        .context("connecting to PipeWire core")?;
    let registry = Rc::new(core.get_registry().context("getting PipeWire registry")?);
    let registry_weak = Rc::downgrade(&registry);

    let bound_nodes = Rc::new(RefCell::new(HashMap::<u32, BoundNode>::new()));
    let snapshots = Rc::new(RefCell::new(HashMap::<u32, AudioNodeSnapshot>::new()));

    let _registry_listener = registry
        .add_listener_local()
        .global({
            let event_bus = event_bus.clone();
            let bound_nodes = Rc::clone(&bound_nodes);
            let snapshots = Rc::clone(&snapshots);
            move |global| {
                if global.type_ != ObjectType::Node || !is_audio_node(global.props.as_ref()) {
                    return;
                }

                let Some(registry) = registry_weak.upgrade() else {
                    return;
                };

                let node: Node = match registry.bind(global) {
                    Ok(node) => node,
                    Err(error) => {
                        warn!("binding PipeWire node {} failed: {error}", global.id);
                        return;
                    }
                };

                if let Some(snapshot) = snapshot_from_global(global) {
                    emit_audio_node(&event_bus, &snapshot);
                    snapshots.borrow_mut().insert(snapshot.id, snapshot);
                }

                let node_id = global.id;
                let info_listener = node
                    .add_listener_local()
                    .info({
                        let event_bus = event_bus.clone();
                        let snapshots = Rc::clone(&snapshots);
                        move |info| {
                            if let Some(snapshot) = snapshot_from_info(info) {
                                emit_audio_node(&event_bus, &snapshot);
                                snapshots.borrow_mut().insert(snapshot.id, snapshot);
                            }
                        }
                    })
                    .register();

                let proxy_listener = node
                    .upcast_ref()
                    .add_listener_local()
                    .removed({
                        let bound_nodes = Rc::clone(&bound_nodes);
                        move || {
                            bound_nodes.borrow_mut().remove(&node_id);
                        }
                    })
                    .register();

                bound_nodes.borrow_mut().insert(
                    node_id,
                    BoundNode {
                        _node: node,
                        _listener: info_listener,
                        _proxy_listener: proxy_listener,
                    },
                );
            }
        })
        .global_remove({
            let event_bus = event_bus.clone();
            let bound_nodes = Rc::clone(&bound_nodes);
            let snapshots = Rc::clone(&snapshots);
            move |id| {
                bound_nodes.borrow_mut().remove(&id);
                if let Some(previous) = snapshots.borrow_mut().remove(&id) {
                    event_bus.emit(
                        "audio:node",
                        serde_json::json!({
                            "id": previous.id,
                            "name": previous.name,
                            "state": "removed",
                            "volume": previous.volume,
                            "muted": previous.muted,
                        }),
                    );
                }
            }
        })
        .register();

    debug!("audio monitor attached to PipeWire registry");
    mainloop.run();
    Ok(())
}

fn is_audio_node(props: Option<&DictRef>) -> bool {
    props
        .and_then(|props| props.get("media.class"))
        .map(|class| class == "Audio/Sink" || class == "Audio/Source")
        .unwrap_or(false)
}

fn snapshot_from_global(global: &GlobalObject<&DictRef>) -> Option<AudioNodeSnapshot> {
    let props = global.props.as_ref()?;
    if !is_audio_node(Some(props)) {
        return None;
    }

    Some(AudioNodeSnapshot {
        id: global.id,
        name: node_name(props),
        state: node_state_from_props(props),
        volume: node_volume(props),
        muted: node_muted(props),
    })
}

fn snapshot_from_info(info: &NodeInfoRef) -> Option<AudioNodeSnapshot> {
    let props = info.props()?;
    if !is_audio_node(Some(props)) {
        return None;
    }

    Some(AudioNodeSnapshot {
        id: info.id(),
        name: node_name(props),
        state: node_state(info.state()),
        volume: node_volume(props),
        muted: node_muted(props),
    })
}

fn emit_audio_node(event_bus: &EventBus, snapshot: &AudioNodeSnapshot) {
    event_bus.emit(
        "audio:node",
        serde_json::json!({
            "id": snapshot.id,
            "name": snapshot.name,
            "state": snapshot.state,
            "volume": snapshot.volume,
            "muted": snapshot.muted,
        }),
    );
}

fn node_name(props: &DictRef) -> String {
    [
        "node.description",
        "media.name",
        "node.nick",
        "node.name",
        "application.name",
    ]
    .into_iter()
    .find_map(|key| props.get(key))
    .unwrap_or("unknown")
    .to_string()
}

fn node_state_from_props(props: &DictRef) -> String {
    props
        .get("node.state")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "idle".to_string())
}

fn node_state(state: NodeState<'_>) -> String {
    match state {
        NodeState::Error(_) => "error",
        NodeState::Creating => "creating",
        NodeState::Suspended => "suspended",
        NodeState::Idle => "idle",
        NodeState::Running => "running",
    }
    .to_string()
}

fn node_volume(props: &DictRef) -> f64 {
    ["volume", "audio.volume", "channelmix.volume", "node.volume"]
        .into_iter()
        .find_map(|key| props.parse::<f64>(key).and_then(Result::ok))
        .unwrap_or(1.0)
}

fn node_muted(props: &DictRef) -> bool {
    ["mute", "audio.mute", "node.mute"]
        .into_iter()
        .find_map(|key| props.parse::<bool>(key).and_then(Result::ok))
        .unwrap_or(false)
}
