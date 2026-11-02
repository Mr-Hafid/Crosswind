use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use notify::{EventKind, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::receiver::{GlowPhase, emit_glow};
use crate::thumbnail;

const QUARANTINE_ATTRIBUTE: &str = "com.apple.quarantine";
const AIRDROP_AGENT: &str = "sharingd";
const FRESH_ARRIVAL_WINDOW: Duration = Duration::from_secs(120);
const ALLOWED_CLOCK_SKEW: Duration = Duration::from_secs(60);
const BATCH_WINDOW: Duration = Duration::from_millis(800);
const INCOMING_TO_RECEIVED_DELAY: Duration = Duration::from_millis(700);
const SYSTEM_TRANSFER_ID: i64 = -2;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemAirDropFile {
    name: String,
    file_type: String,
    size: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemAirDropArrival {
    id: i64,
    sender_name: String,
    sender_model: String,
    files: Vec<SystemAirDropFile>,
    urls: Vec<String>,
    auto_accepted: bool,
    thumbnail: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemAirDropFinished {
    id: i64,
    paths: Vec<String>,
    urls: Vec<String>,
    reason: Option<String>,
    sender_name: String,
}

pub fn watch_downloads(app: &AppHandle) {
    let Ok(downloads_dir) = app.path().download_dir() else {
        return;
    };
    let app = app.clone();
    std::thread::spawn(move || run_watcher(app, downloads_dir));
}

fn run_watcher(app: AppHandle, downloads_dir: PathBuf) {
    let (event_sender, event_receiver) = mpsc::channel();
    let Ok(mut watcher) = notify::recommended_watcher(event_sender) else {
        return;
    };
    if watcher
        .watch(&downloads_dir, RecursiveMode::NonRecursive)
        .is_err()
    {
        return;
    }

    let mut arrival_filter = ArrivalFilter::default();
    loop {
        let Ok(first_event) = event_receiver.recv() else {
            return;
        };
        let mut candidate_paths: HashSet<PathBuf> = paths_from(first_event).into_iter().collect();
        while let Ok(next_event) = event_receiver.recv_timeout(BATCH_WINDOW) {
            candidate_paths.extend(paths_from(next_event));
        }
        let candidates = candidate_paths
            .into_iter()
            .filter_map(|path| {
                let canonical_path = path.canonicalize().ok()?;
                let quarantine = read_airdrop_quarantine(&canonical_path)?;
                Some((canonical_path, quarantine))
            })
            .collect();
        let arrived_paths = arrival_filter.admit_batch(candidates, SystemTime::now());
        if !arrived_paths.is_empty() {
            announce_arrival(&app, arrived_paths);
        }
    }
}

#[derive(Default)]
struct ArrivalFilter {
    announced_transfer_ids: HashSet<String>,
    recently_announced_paths: HashMap<PathBuf, SystemTime>,
}

impl ArrivalFilter {
    fn admit_batch(
        &mut self,
        candidates: Vec<(PathBuf, AirDropQuarantine)>,
        now: SystemTime,
    ) -> Vec<PathBuf> {
        self.recently_announced_paths
            .retain(|_, announced_at| is_within_fresh_window(*announced_at, now));
        let admitted: Vec<(PathBuf, AirDropQuarantine)> = candidates
            .into_iter()
            .filter(|(path, quarantine)| {
                quarantine.is_fresh(now)
                    && !self.announced_transfer_ids.contains(&quarantine.transfer_id)
                    && !self.recently_announced_paths.contains_key(path)
            })
            .collect::<HashMap<_, _>>()
            .into_iter()
            .collect();
        admitted
            .into_iter()
            .map(|(path, quarantine)| {
                self.announced_transfer_ids.insert(quarantine.transfer_id);
                self.recently_announced_paths.insert(path.clone(), now);
                path
            })
            .collect()
    }
}

fn is_within_fresh_window(moment: SystemTime, now: SystemTime) -> bool {
    match now.duration_since(moment) {
        Ok(age) => age <= FRESH_ARRIVAL_WINDOW,
        Err(clock_skew) => clock_skew.duration() <= ALLOWED_CLOCK_SKEW,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct AirDropQuarantine {
    received_at: SystemTime,
    transfer_id: String,
}

impl AirDropQuarantine {
    fn parse(value: &str) -> Option<Self> {
        let mut fields = value.split(';');
        let _flags = fields.next()?;
        let received_at_seconds = u64::from_str_radix(fields.next()?, 16).ok()?;
        let agent = fields.next()?;
        let transfer_id = fields.next()?.trim();
        if agent != AIRDROP_AGENT || transfer_id.is_empty() {
            return None;
        }
        Some(AirDropQuarantine {
            received_at: UNIX_EPOCH + Duration::from_secs(received_at_seconds),
            transfer_id: transfer_id.to_owned(),
        })
    }

    fn is_fresh(&self, now: SystemTime) -> bool {
        is_within_fresh_window(self.received_at, now)
    }
}

fn paths_from(event: notify::Result<notify::Event>) -> Vec<PathBuf> {
    match event {
        Ok(event) if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) => {
            event.paths
        }
        _ => Vec::new(),
    }
}

fn read_airdrop_quarantine(path: &Path) -> Option<AirDropQuarantine> {
    if !path.is_file() {
        return None;
    }
    let value = xattr::get(path, QUARANTINE_ATTRIBUTE).ok().flatten()?;
    AirDropQuarantine::parse(&String::from_utf8_lossy(&value))
}

fn announce_arrival(app: &AppHandle, paths: Vec<PathBuf>) {
    let sender_name = "AirDrop".to_owned();
    let files = paths
        .iter()
        .map(|path| SystemAirDropFile {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            file_type: path
                .extension()
                .map(|extension| extension.to_string_lossy().to_lowercase())
                .unwrap_or_default(),
            size: std::fs::metadata(path).ok().map(|metadata| metadata.len()),
        })
        .collect();
    let _ = app.emit(
        "incoming-request",
        SystemAirDropArrival {
            id: SYSTEM_TRANSFER_ID,
            sender_name: sender_name.clone(),
            sender_model: "macOS".to_owned(),
            files,
            urls: Vec::new(),
            auto_accepted: true,
            thumbnail: paths.first().and_then(|path| thumbnail::data_url_for_file(path)),
        },
    );
    emit_glow(app, GlowPhase::Incoming);
    std::thread::sleep(INCOMING_TO_RECEIVED_DELAY);
    let finished = SystemAirDropFinished {
        id: SYSTEM_TRANSFER_ID,
        paths: paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        urls: Vec::new(),
        reason: None,
        sender_name,
    };
    let _ = app.emit("system-airdrop-received", finished.clone());
    let _ = app.emit(
        "transfer-finished",
        SystemAirDropFinished {
            paths: Vec::new(),
            ..finished
        },
    );
    emit_glow(app, GlowPhase::Received);
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIVED_AT_SECONDS: u64 = 0x6ae7d8d9;
    const AIRDROP_VALUE: &str = "0081;6ae7d8d9;sharingd;CBFC944D-75B0-4F62-8FB9-1B190CA7D34D";

    fn received_at() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(RECEIVED_AT_SECONDS)
    }

    fn quarantined_file(name: &str, value: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("crosswind-{}-{name}", std::process::id()));
        std::fs::write(&path, b"photo").unwrap();
        xattr::set(&path, QUARANTINE_ATTRIBUTE, value.as_bytes()).unwrap();
        path
    }

    #[test]
    fn airdrop_quarantine_parses_time_and_transfer_id() {
        assert_eq!(
            AirDropQuarantine::parse(AIRDROP_VALUE),
            Some(AirDropQuarantine {
                received_at: received_at(),
                transfer_id: "CBFC944D-75B0-4F62-8FB9-1B190CA7D34D".to_owned(),
            })
        );
    }

    #[test]
    fn other_agents_are_not_airdrop() {
        assert_eq!(AirDropQuarantine::parse("0081;6ae7d8d9;Google Chrome;ABC"), None);
        assert_eq!(AirDropQuarantine::parse("0081;6ae7d8d9;sharingd;"), None);
        assert_eq!(AirDropQuarantine::parse("garbage"), None);
    }

    #[test]
    fn arrival_is_fresh_only_within_the_window() {
        let quarantine = AirDropQuarantine::parse(AIRDROP_VALUE).unwrap();
        assert!(quarantine.is_fresh(received_at() + Duration::from_secs(5)));
        assert!(quarantine.is_fresh(received_at() + FRESH_ARRIVAL_WINDOW));
        assert!(!quarantine.is_fresh(received_at() + FRESH_ARRIVAL_WINDOW + Duration::from_secs(1)));
        assert!(!quarantine.is_fresh(received_at() + Duration::from_secs(60 * 60 * 24)));
        assert!(quarantine.is_fresh(received_at() - Duration::from_secs(30)));
    }

    fn quarantine_with_id(transfer_id: &str) -> AirDropQuarantine {
        AirDropQuarantine {
            received_at: received_at(),
            transfer_id: transfer_id.to_owned(),
        }
    }

    #[test]
    fn same_file_retagged_with_a_new_transfer_id_is_announced_once() {
        let mut filter = ArrivalFilter::default();
        let path = PathBuf::from("/Users/someone/Downloads/IMG_9111 3.PNG");
        let now = received_at() + Duration::from_secs(1);
        let first = filter.admit_batch(vec![(path.clone(), quarantine_with_id("FIRST"))], now);
        let second = filter.admit_batch(
            vec![(path.clone(), quarantine_with_id("SECOND"))],
            now + Duration::from_secs(2),
        );
        assert_eq!(first, vec![path]);
        assert!(second.is_empty());
    }

    #[test]
    fn copy_of_a_fresh_arrival_is_not_announced() {
        let mut filter = ArrivalFilter::default();
        let now = received_at() + Duration::from_secs(1);
        let original = PathBuf::from("/Users/someone/Downloads/photo.png");
        let copy = PathBuf::from("/Users/someone/Downloads/photo copy.png");
        filter.admit_batch(vec![(original, quarantine_with_id("SHARED"))], now);
        let announced = filter.admit_batch(
            vec![(copy, quarantine_with_id("SHARED"))],
            now + Duration::from_secs(5),
        );
        assert!(announced.is_empty());
    }

    #[test]
    fn duplicate_events_in_one_batch_announce_the_file_once() {
        let mut filter = ArrivalFilter::default();
        let path = PathBuf::from("/Users/someone/Downloads/photo.png");
        let announced = filter.admit_batch(
            vec![
                (path.clone(), quarantine_with_id("ONE")),
                (path.clone(), quarantine_with_id("ONE")),
            ],
            received_at() + Duration::from_secs(1),
        );
        assert_eq!(announced, vec![path]);
    }

    #[test]
    fn several_files_of_one_transfer_are_announced_together() {
        let mut filter = ArrivalFilter::default();
        let first = PathBuf::from("/Users/someone/Downloads/a.png");
        let second = PathBuf::from("/Users/someone/Downloads/b.png");
        let mut announced = filter.admit_batch(
            vec![
                (first.clone(), quarantine_with_id("BATCH")),
                (second.clone(), quarantine_with_id("BATCH")),
            ],
            received_at() + Duration::from_secs(1),
        );
        announced.sort();
        assert_eq!(announced, vec![first, second]);
    }

    #[test]
    fn quarantine_is_read_from_the_file() {
        let airdropped = quarantined_file("airdrop.png", AIRDROP_VALUE);
        let downloaded = quarantined_file("browser.png", "0081;6ae7d8d9;Google Chrome;ABC");
        assert!(read_airdrop_quarantine(&airdropped).is_some());
        assert!(read_airdrop_quarantine(&downloaded).is_none());
        std::fs::remove_file(airdropped).unwrap();
        std::fs::remove_file(downloaded).unwrap();
    }
}
