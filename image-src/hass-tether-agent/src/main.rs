use env_logger;
use hash_map_diff::hash_map_diff;
use hass_rs::{client, WSEvent};
use lazy_static::lazy_static;
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, env::var};
use tether_agent::{PlugOptionsBuilder, TetherAgentOptionsBuilder};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use url::Url;

lazy_static! {
    static ref TOKEN: String = var("SUPERVISOR_TOKEN")
        .expect("please set up the SUPERVISOR_TOKEN env variable before running this");
    static ref HOST: String = var("HOST").expect("Missing HOST env variable");
    static ref PORT: u16 = var("PORT")
        .unwrap()
        .parse()
        .expect("Missing PORT env variable");
    static ref USERNAME: String = var("USERNAME").expect("Missing USERNAME env variable");
    static ref PASSWORD: String = var("PASSWORD").expect("Missing PASSWORD env variable");
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EventStruct {
    entity_id: String,
    event_type: String,
    attribute: String,
    state: Value,
}

fn addon_log(thread: &str, message: &str) {
    println!("hass-tether-agent:: {}: {}", thread, message);
}

async fn setup_tether_agent(mut receiver: UnboundedReceiver<EventStruct>) {
    let mut plugs = HashMap::new();

    addon_log("Tether", "Creating Tether options");

    let agent_options = TetherAgentOptionsBuilder::new("homeassistant")
        .host(Some(&HOST))
        .port(Some(*PORT))
        .username(Some(&USERNAME))
        .password(Some(&PASSWORD));

    addon_log("Tether", "Connecting to Tether");
    let agent = agent_options
        .clone()
        .build()
        .expect("failed to connect Tether");

    addon_log("Tether", "Connected to Tether");

    loop {
        match receiver.recv().await {
            Some(data) => {
                if !agent.is_connected() {
                    match agent.connect() {
                        Ok(_) => addon_log("Tether", "Tether reconnected"),
                        Err(e) => {
                            addon_log(
                                "Tether",
                                format!("Error reconnecting to Tether: {}", e).as_str(),
                            );
                            return;
                        }
                    }
                }

                let name = format!("{}/{}", data.entity_id.clone(), data.attribute);
                let topic = format!("{}/{}", "homeassistant", name);
                if !plugs.contains_key(&name) {
                    plugs.insert(
                        name.clone(),
                        PlugOptionsBuilder::create_output(&data.attribute)
                            .topic(Some(&topic))
                            .build(&agent)
                            .expect("Failed to create Tether plug"),
                    );
                }

                let plug = plugs.get(&name).unwrap();

                match agent.encode_and_publish(&plug, data.state) {
                    Ok(_) => continue,
                    Err(e) => {
                        addon_log("Tether", format!("Error publishing: {}", e).as_str());
                        return;
                    }
                }
            }
            None => {
                continue;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    addon_log("Main", "Starting addon");
    env_logger::init();

    addon_log("Main", "Launching services");
    let (sender, receiver) = unbounded_channel();
    addon_log("Main", "Connecting to Tether broker");
    tokio::spawn(async move { setup_tether_agent(receiver).await });

    addon_log("Main", "Connecting to Home Assistant websocket client");

    let url = Url::parse("ws://supervisor/core/websocket").unwrap();
    let mut client = client::connect_to_url(url).await?;
    client.auth_with_longlivedtoken(&*TOKEN).await?;
    addon_log("Main", "Connected to Home Assistant websocket client");

    let closure = move |item: WSEvent| {
        let old_state = match item.event.data.old_state {
            Some(entity) => entity,
            None => return,
        };
        let new_state = match item.event.data.new_state {
            Some(entity) => entity,
            None => return,
        };

        let old_state: HashMap<String, Value> =
            match serde_json::from_str(&old_state.attributes.to_string()) {
                Ok(state) => state,
                Err(e) => {
                    addon_log(
                        "HASS websocket connection",
                        format!("Error parsing old state json: {}", e).as_str(),
                    );
                    return;
                }
            };
        let new_state: HashMap<String, Value> =
            match serde_json::from_str(&new_state.attributes.to_string()) {
                Ok(state) => state,
                Err(e) => {
                    addon_log(
                        "HASS websocket connection",
                        format!("Error parsing new state json: {}", e).as_str(),
                    );
                    return;
                }
            };
        let received_diff = hash_map_diff(&old_state, &new_state);

        for (key, value) in received_diff.updated.into_iter() {
            let custom_message = EventStruct {
                entity_id: item.event.data.entity_id.clone(),
                event_type: item.event.event_type.clone(),
                attribute: key.clone(),
                state: value.clone(),
            };

            sender.send(custom_message).unwrap_or_else(|error| {
                addon_log(
                    "HASS websocket connection",
                    format!("Error sending diff message to Tether: {}", error).as_str(),
                )
            });
        }
    };

    match client.subscribe_event("state_changed", closure).await {
        Ok(v) => addon_log(
            "HASS websocket connection",
            format!("Subscribed to state change events: {}", v).as_str(),
        ),
        Err(err) => addon_log(
            "HASS websocket connection",
            format!("Oh no, an error: {}", err).as_str(),
        ),
    }
    loop {}
}
