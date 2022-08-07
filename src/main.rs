use futures::{StreamExt, SinkExt};
use log::*;
use rand::Rng;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Error, WebSocketStream};
use serde::{Serialize, Deserialize};
use std::convert::{From, TryFrom};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Sender, Receiver};

struct Clients {
    clients: HashMap<i64, Sender<Option<String>>>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TTRequest {
    pub action: String,
    pub gameId: String,
    pub sender: i32,
    pub recipients: Vec<i32>,
    pub messageId: i16,
    pub payload: String
}

pub struct PlayerData {
    pub playerId: i64,
    pub connectionId: i64
}

pub struct GameData {
    pub players: Vec<PlayerData>
}

pub struct Database {
    // Map from ConnectionID to GameID
    pub connections: HashMap<i64, String>,
    pub games: HashMap<String, GameData>
}

impl Database {
    pub fn new() -> Database {
        Database { 
            connections: HashMap::new(),
            games: HashMap::new()
        }
    }
}

async fn process_host(db: &mut Database, conns: &Arc<Mutex<Clients>>, sender_id: i64, mut msg: TTRequest) {
    println!("Hosting {:?}", msg);
    let new_game_id: String = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(4)
        .map(char::from)
        .collect();

    db.connections.insert(sender_id, new_game_id.clone());
    db.games.insert(new_game_id.clone(), GameData { players: vec!(PlayerData { playerId: 0, connectionId: sender_id }) });

    let mut parsed_payload: serde_json::Value = serde_json::from_str(&msg.payload).unwrap();
    parsed_payload["gameId"] = serde_json::Value::String(new_game_id.clone());
    msg.payload = serde_json::to_string(&parsed_payload).unwrap();
    msg.gameId = new_game_id;

    if let Some(s) = conns.lock().await.clients.get(&sender_id) {
        s.send(Some(serde_json::to_string(&msg).unwrap())).await.unwrap();
    }
}

async fn process_add_guest(db: &mut Database, conns: &Arc<Mutex<Clients>>, sender_id: i64, mut msg: TTRequest) {
    println!("Add guest {:?}", &msg);
    let mut body_json: serde_json::Value = serde_json::from_str(&msg.payload).unwrap();
    let new_pid = msg.recipients.first().unwrap();
    let guest_connection_id = body_json.get("arg").unwrap().as_str().unwrap().parse::<i64>().unwrap();

    if *new_pid > 0 {
        let game = db.games.get_mut(&msg.gameId).unwrap();
        game.players.push(PlayerData { playerId: (*new_pid).into(), connectionId: guest_connection_id });
        db.connections.insert(guest_connection_id, msg.gameId.clone());
        let mut m = serde_json::Map::new();
        m.insert("gameId".to_owned(), serde_json::Value::String(msg.gameId.clone()));
        m.insert("pId".to_owned(), serde_json::Value::String(new_pid.to_string()));
        body_json = serde_json::Value::Object(m);
    }
    
    msg.payload = serde_json::to_string(&body_json).unwrap();
    if let Some(s) = conns.lock().await.clients.get(&guest_connection_id) {
        s.send(Some(serde_json::to_string(&msg).unwrap())).await.unwrap();
    }
}

async fn process_disconnect(db: &mut Database, conns: &Arc<Mutex<Clients>>, sender_id: i64) {
    println!("Disconnect {:?}", &sender_id);
    if let Some(g) = db.connections.remove(&sender_id) {
        if let Some(s) = db.games.get_mut(&g) {
            s.players.retain(|x| x.connectionId != sender_id);
            if s.players.len() == 0 {
                db.games.remove(&g);
            }
        }
    }

    conns.lock().await.clients.get(&sender_id).unwrap().send(None).await;
    // Tell all other players still in the game their host disconnected
    // TODO
}

async fn process_join(db: &mut Database, connections: &Arc<Mutex<Clients>>, sender_id: i64, msg: TTRequest) {
    println!("Join {:?}", &msg);
    let body_json: serde_json::Value = serde_json::from_str(&msg.payload).unwrap();
    let room = body_json.get("arg").unwrap().to_string();

    if let Some(s) = db.games.get(&room) {
        let host = s.players.iter().filter(|x| x.playerId == 0).next().unwrap();
        let val = Some(serde_json::to_string(&msg).unwrap());
        connections.lock().await.clients.get(&host.connectionId).unwrap().send(val).await;
    } else {
        let mut clone = msg.clone();
        clone.messageId += 1;
        let val = Some(serde_json::to_string(&clone).unwrap());
        connections.lock().await.clients.get(&sender_id).unwrap().send(val).await;
    }
}

async fn process_send(db: &mut Database, connections: &Arc<Mutex<Clients>>, sender_id: i64, msg: TTRequest) {
    let resp = serde_json::to_string(&msg).unwrap();
    let mut clients = connections.lock().await;
    for recip in msg.recipients {
        if let Some(s) = clients.clients.get_mut(&(recip.into())) {
            s.send(Some(resp.clone())).await.unwrap();
        } else {
            println!("Recipient not found");
        }
    }
}

async fn process_server(connections: Arc<Mutex<Clients>>, mut recv: Receiver<(i64, Option<TTRequest>)>) {
    let mut db = Database::new();
    while let Some((sender_id, req)) = recv.recv().await {
        if let Some(request) = req {
            match request.action.as_str() {
                "host" => process_host(&mut db, &connections, sender_id, request).await,
                "join" => process_join(&mut db, &connections, sender_id, request).await,
                "addGuest" => process_add_guest(&mut db, &connections, sender_id, request).await,
                "send" => process_send(&mut db, &connections, sender_id, request).await,
                _ => { continue; }
            }
        } else {
            process_disconnect(&mut db, &connections, sender_id).await;
        }
    }
}

async fn process_client(client_id: i64, mut socket: WebSocketStream<TcpStream>, mut receiver: Receiver<Option<String>>, sender: Sender<(i64, Option<TTRequest>)>) {   
    loop {
        tokio::select! {
            inbound = socket.next() => {
                let msg = inbound.unwrap().unwrap();
                if msg.is_text() {
                    let msg2: TTRequest = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
                    sender.send((client_id, Some(msg2))).await.unwrap();
                }
            },
            outbound = receiver.recv() => {
                if let Some(s) = outbound.unwrap() {
                    socket.send(tungstenite::Message::Text(s)).await;
                } else {
                    socket.close(None).await;
                    break;
                }
            }
            else => { break; }
        }
    }
    println!("End client loop")
}

async fn accept(client_id: i64, client_recv: Receiver<Option<String>>, sender: Sender<(i64, Option<TTRequest>)>, stream: TcpStream) {
    let ws_stream = match accept_async(stream).await {
        Ok(o) => o,
        Err(e) => {
            println!("Error: {:?}", e);
            return;
        }
    };

    println!("process client");
    process_client(client_id, ws_stream, client_recv, sender).await;
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:9002";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    info!("Listening on: {}", addr);

    let (cmd_send, cmd_recv) = tokio::sync::mpsc::channel(1000);
    let clients = Arc::new(Mutex::new(Clients { clients: HashMap::new() }));
    tokio::spawn(process_server(clients.clone(), cmd_recv));

    let mut id: i64 = 0;
    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream.peer_addr().expect("connected streams should have a peer address");
        println!("Peer address: {}", peer);

        let (cc_s, cc_r) = tokio::sync::mpsc::channel(100);
        clients.lock().await.clients.insert(id, cc_s);
        
        tokio::spawn(accept(id, cc_r, cmd_send.clone(), stream));
        id += 1;
    }
}
