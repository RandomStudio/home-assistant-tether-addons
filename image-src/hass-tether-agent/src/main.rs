use env_logger;
use hash_map_diff::hash_map_diff;
use hass_rs::{client, WSEvent};
use lazy_static::lazy_static;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    env::var,
    sync::mpsc::{self, Receiver},
    thread::{self},
};
use tether_agent::{PlugOptionsBuilder, TetherAgentOptionsBuilder};
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

fn setup_tether_agent(receiver: Receiver<EventStruct>) {
    println!("Setting Tether up");

    println!("Tether conneted");

    let mut plugs = HashMap::new();

    let agent_options = TetherAgentOptionsBuilder::new("homeassistant")
        .host(&HOST)
        .port(*PORT)
        .username(&USERNAME)
        .password(&PASSWORD);

    let agent = agent_options
        .clone()
        .build()
        .expect("failed to connect Tether");

    loop {
        if !agent.is_connected() {
            match agent.connect(&agent_options) {
                Ok(_) => println!("Tether reconnected"),
                Err(e) => println!("Error reconnecting to Tether: {}", e),
            }
        }

        let received = receiver.recv();
        if received.is_err() {
            println!("Error receiving message {}", received.err().unwrap());
            return;
        }

        let data = received.unwrap();

        let name = format!("{}/{}", data.entity_id.clone(), data.attribute);
        let topic = format!("{}/{}", "homeassistant", name);
        if !plugs.contains_key(&name) {
            plugs.insert(
                name.clone(),
                PlugOptionsBuilder::create_output(&data.attribute)
                    .topic(&topic)
                    .build(&agent)
                    .expect("failed to create output"),
            );
        }

        let plug = plugs.get(&name).unwrap();

        match agent.encode_and_publish(&plug, data.state) {
            Ok(_) => continue,
            Err(e) => println!("Error publishing: {}", e),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    println!("Launching services");

    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || setup_tether_agent(receiver));

    println!("Creating the Websocket Client and Authenticate the session");
    let url = Url::parse("ws://supervisor/core/websocket").unwrap();
    let mut client = client::connect_to_url(url).await?;
    client.auth_with_longlivedtoken(&*TOKEN).await?;

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
                    println!("Error parsing json: {}", e);
                    return;
                }
            };
        let new_state: HashMap<String, Value> =
            match serde_json::from_str(&new_state.attributes.to_string()) {
                Ok(state) => state,
                Err(e) => {
                    println!("Error parsing json: {}", e);
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

            sender.send(custom_message).unwrap();
        }
    };

    match client.subscribe_event("state_changed", closure).await {
        Ok(v) => println!("Event subscribed: {}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }

    loop {}
}
