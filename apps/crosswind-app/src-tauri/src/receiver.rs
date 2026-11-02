use std::collections::HashMap;
use std::net::SocketAddrV6;
use std::path::PathBuf;
use std::sync::Mutex;

use crosswind_airdrop::driver::mdns::Mdns;
use crosswind_airdrop::driver::receiver::{Options, Receiver, ReceiverEvent, Request};
use crosswind_airdrop::driver::tls::Identity;
use crosswind_airdrop::proto::mdns::{self, Service};
use crosswind_airdrop::proto::messages::Offer;
use crosswind_airdrop::proto::receiver::Config;
use crosswind_core::Platform;
use crosswind_core::link::{LinkInterface, default_interface_name};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc;

use crate::thumbnail;

const RECEIVE_PORT: u16 = 8771;
const RECEIVER_FLAGS: u32 = 0x089;

#[derive(Default)]
pub struct ReceiverState {
    running: Mutex<Option<RunningReceiver>>,
    pending_requests: Mutex<HashMap<u64, Request>>,
}

struct RunningReceiver {
    tasks: Vec<JoinHandle<()>>,
    status: ReceiverStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiverStatus {
    display_name: String,
    interface_name: String,
    address: String,
    service_id: String,
    download_dir: String,
    auto_accept: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceChoice {
    name: String,
    address: String,
    is_default: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IncomingRequest {
    id: u64,
    sender_name: String,
    sender_model: String,
    files: Vec<OfferedFile>,
    urls: Vec<String>,
    auto_accepted: bool,
    thumbnail: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OfferedFile {
    name: String,
    file_type: String,
    size: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransferUpdate {
    id: u64,
    paths: Vec<String>,
    urls: Vec<String>,
    reason: Option<String>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GlowPhase {
    Nearby,
    Incoming,
    Received,
    Idle,
}

pub fn emit_glow(app: &AppHandle, phase: GlowPhase) {
    let _ = app.emit("glow", phase);
}

#[tauri::command]
pub fn list_interfaces() -> Vec<InterfaceChoice> {
    let preferred_name = default_interface_name();
    LinkInterface::list_usable()
        .into_iter()
        .map(|interface| InterfaceChoice {
            is_default: interface.name == preferred_name,
            address: interface.link_local_addr.to_string(),
            name: interface.name,
        })
        .collect()
}

#[tauri::command]
pub fn default_download_dir(app: AppHandle) -> Result<String, String> {
    app.path()
        .download_dir()
        .map(|dir| dir.to_string_lossy().into_owned())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn receiver_status(state: State<'_, ReceiverState>) -> Option<ReceiverStatus> {
    state
        .running
        .lock()
        .ok()?
        .as_ref()
        .map(|running| running.status.clone())
}

#[tauri::command]
pub async fn start_receiving(
    app: AppHandle,
    state: State<'_, ReceiverState>,
    interface_name: String,
    download_dir: String,
    auto_accept: bool,
) -> Result<ReceiverStatus, String> {
    stop_running_receiver(&state);
    let interface = LinkInterface::lookup(&interface_name).map_err(|error| error.to_string())?;
    let identity_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&identity_dir).map_err(|error| error.to_string())?;
    let identity = Identity::load_or_generate(&identity_dir).map_err(|error| error.to_string())?;

    let display_name = local_display_name();
    let mut config = Config::new(display_name.clone());
    config.model_name = Platform::current().display_name().into();
    let listen_addr =
        SocketAddrV6::new(interface.link_local_addr, RECEIVE_PORT, 0, interface.index);
    let (receiver, events) = Receiver::bind(
        listen_addr,
        &identity,
        Options::new(config, PathBuf::from(&download_dir)),
    )
    .await
    .map_err(|error| format!("cannot listen on {}: {error}", interface.name))?;

    let service_id = random_service_id()?;
    let service = Service {
        instance: service_id.clone(),
        host: format!("crosswind-{service_id}"),
        port: RECEIVE_PORT,
        addr: interface.link_local_addr,
        flags: RECEIVER_FLAGS,
    };
    let registration = Mdns::new(interface.index)
        .register(service)
        .await
        .map_err(|error| format!("cannot announce over mDNS: {error}"))?;

    let receiver_task = tauri::async_runtime::spawn(receiver.run());
    let event_task = tauri::async_runtime::spawn(forward_events(
        app.clone(),
        events,
        registration,
        auto_accept,
    ));

    let status = ReceiverStatus {
        display_name,
        interface_name: interface.name,
        address: interface.link_local_addr.to_string(),
        service_id,
        download_dir,
        auto_accept,
    };
    if let Ok(mut running) = state.running.lock() {
        *running = Some(RunningReceiver {
            tasks: vec![receiver_task, event_task],
            status: status.clone(),
        });
    }
    Ok(status)
}

#[tauri::command]
pub fn stop_receiving(app: AppHandle, state: State<'_, ReceiverState>) {
    stop_running_receiver(&state);
    emit_glow(&app, GlowPhase::Idle);
}

#[tauri::command]
pub fn answer_request(
    app: AppHandle,
    state: State<'_, ReceiverState>,
    id: u64,
    accept: bool,
) -> Result<(), String> {
    let request = state
        .pending_requests
        .lock()
        .map_err(|error| error.to_string())?
        .remove(&id)
        .ok_or_else(|| "this request is no longer waiting".to_owned())?;
    if accept {
        request.accept();
    } else {
        request.decline();
        emit_glow(&app, GlowPhase::Idle);
    }
    Ok(())
}

#[tauri::command]
pub fn preview_glow(app: AppHandle, phase: GlowPhase) {
    emit_glow(&app, phase);
}

#[tauri::command]
pub fn reveal_received(path: String) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|error| error.to_string())
}

fn stop_running_receiver(state: &ReceiverState) {
    if let Ok(mut running) = state.running.lock() {
        if let Some(previous) = running.take() {
            previous.tasks.iter().for_each(|task| task.abort());
        }
    }
    if let Ok(mut pending) = state.pending_requests.lock() {
        pending.drain().for_each(|(_, request)| request.decline());
    }
}

async fn forward_events(
    app: AppHandle,
    mut events: mpsc::Receiver<ReceiverEvent>,
    registration: crosswind_airdrop::driver::mdns::Registration,
    auto_accept: bool,
) {
    let _registration = registration;
    while let Some(event) = events.recv().await {
        match event {
            ReceiverEvent::Discovered { peer, .. } => {
                let _ = app.emit("peer-seen", peer.to_string());
                emit_glow(&app, GlowPhase::Nearby);
            }
            ReceiverEvent::Request(request) => {
                let incoming = describe_request(&request, auto_accept);
                let _ = app.emit("incoming-request", incoming);
                emit_glow(&app, GlowPhase::Incoming);
                if auto_accept {
                    request.accept();
                } else if let Ok(mut pending) = app.state::<ReceiverState>().pending_requests.lock()
                {
                    pending.insert(request.id, request);
                }
            }
            ReceiverEvent::Received { id, paths } => {
                if let Some(first_path) = paths.first().cloned() {
                    let thumbnail_app = app.clone();
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        thumbnail::emit_card_thumbnail_for_file(&thumbnail_app, &first_path)
                    })
                    .await;
                }
                let update = TransferUpdate {
                    id,
                    paths: paths
                        .iter()
                        .map(|path| path.to_string_lossy().into_owned())
                        .collect(),
                    urls: Vec::new(),
                    reason: None,
                };
                let _ = app.emit("transfer-finished", update);
                emit_glow(&app, GlowPhase::Received);
            }
            ReceiverEvent::Urls { id, urls } => {
                let update = TransferUpdate {
                    id,
                    paths: Vec::new(),
                    urls,
                    reason: None,
                };
                let _ = app.emit("transfer-finished", update);
                emit_glow(&app, GlowPhase::Received);
            }
            ReceiverEvent::Withdrawn { id } => {
                forget_pending(&app, id);
                let _ = app.emit("transfer-withdrawn", id);
                emit_glow(&app, GlowPhase::Idle);
            }
            ReceiverEvent::Failed { id, reason } => {
                forget_pending(&app, id);
                let update = TransferUpdate {
                    id,
                    paths: Vec::new(),
                    urls: Vec::new(),
                    reason: Some(reason),
                };
                let _ = app.emit("transfer-failed", update);
                emit_glow(&app, GlowPhase::Idle);
            }
        }
    }
}

fn forget_pending(app: &AppHandle, id: u64) {
    if let Ok(mut pending) = app.state::<ReceiverState>().pending_requests.lock() {
        pending.remove(&id);
    }
}

fn describe_request(request: &Request, auto_accepted: bool) -> IncomingRequest {
    let (files, urls) = match &request.ask.offer {
        Offer::Files(files) => (
            files
                .iter()
                .map(|file| OfferedFile {
                    name: file.name.clone(),
                    file_type: file.file_type.clone(),
                    size: file.size,
                })
                .collect(),
            Vec::new(),
        ),
        Offer::Urls(urls) => (Vec::new(), urls.clone()),
    };
    IncomingRequest {
        id: request.id,
        sender_name: request.ask.sender_computer_name.clone(),
        sender_model: request.ask.sender_model_name.clone(),
        files,
        urls,
        auto_accepted,
        thumbnail: request
            .ask
            .file_icon
            .as_deref()
            .and_then(thumbnail::data_url_for_sender_icon),
    }
}

fn local_display_name() -> String {
    let host_name = gethostname::gethostname().to_string_lossy().into_owned();
    let short_name = host_name.strip_suffix(".local").unwrap_or(&host_name);
    match Platform::current() {
        Platform::MacOs => format!("{short_name} (Crosswind)"),
        _ => short_name.to_owned(),
    }
}

fn random_service_id() -> Result<String, String> {
    let mut bytes = [0; 6];
    getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
    Ok(mdns::service_id(bytes))
}
