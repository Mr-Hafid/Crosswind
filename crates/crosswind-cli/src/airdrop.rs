use std::io;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use crosswind_airdrop::driver::mdns::Mdns;
use crosswind_airdrop::driver::receiver::{Options, Receiver, ReceiverEvent};
use crosswind_airdrop::driver::sender::{Event, Payload, Sender};
use crosswind_airdrop::driver::tls::Identity;
use crosswind_airdrop::proto::mdns::{self, BrowseEvent, Service};
use crosswind_airdrop::proto::messages::Offer;
use crosswind_airdrop::proto::receiver::Config;
use crosswind_airdrop::proto::sender::{self, Outcome};
use tokio::io::AsyncBufReadExt;

use crosswind_core::link::{LinkInterface, default_interface_name};

const RECEIVE_PORT: u16 = 8771;
const ASK_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(clap::Args)]
pub struct LinkArgs {
    #[arg(long, short, default_value = default_interface_name())]
    iface: String,
    #[arg(long)]
    name: Option<String>,
}

#[derive(clap::Args)]
pub struct ReceiveArgs {
    #[command(flatten)]
    link: LinkArgs,
    #[arg(long, default_value = ".")]
    dir: PathBuf,
    #[arg(long, default_value_t = RECEIVE_PORT)]
    port: u16,
    #[arg(long)]
    accept: bool,
    #[arg(long, default_value = "0x089", value_parser = parse_flags)]
    flags: u32,
}

#[derive(clap::Args)]
pub struct BrowseArgs {
    #[command(flatten)]
    link: LinkArgs,
    #[arg(long, default_value_t = 10)]
    seconds: u64,
}

#[derive(clap::Args)]
pub struct SendArgs {
    #[command(flatten)]
    link: LinkArgs,
    #[arg(long)]
    to: Option<String>,
    #[arg(long, conflicts_with = "to")]
    addr: Option<Ipv6Addr>,
    #[arg(long, default_value_t = RECEIVE_PORT, requires = "addr")]
    port: u16,
    #[arg(long, default_value = "0x089", value_parser = parse_flags, requires = "addr")]
    flags: u32,
    #[arg(long)]
    url: Vec<String>,
    #[arg(long, default_value_t = 30)]
    seconds: u64,
    files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct ListedReceiver {
    instance: String,
    name: String,
    addr: SocketAddrV6,
    flags: u32,
}

impl LinkArgs {
    fn resolve(&self) -> io::Result<(LinkInterface, String)> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let interface = LinkInterface::lookup(&self.iface)?;
        let display_name = self
            .name
            .clone()
            .unwrap_or_else(|| gethostname::gethostname().to_string_lossy().into_owned());
        Ok((interface, display_name))
    }
}

pub async fn receive(args: ReceiveArgs) -> io::Result<ExitCode> {
    let (interface, display_name) = args.link.resolve()?;
    let identity = Identity::load_or_generate(&state_dir()?)?;
    let mut config = Config::new(display_name.clone());
    config.model_name = platform_model_name();
    let listen_addr = SocketAddrV6::new(interface.link_local_addr, args.port, 0, interface.index);
    let (receiver, mut events) =
        Receiver::bind(listen_addr, &identity, Options::new(config, args.dir)).await?;
    tokio::spawn(receiver.run());

    let service_id = random_service_id()?;
    let service = Service {
        instance: service_id.clone(),
        host: format!("crosswind-{service_id}"),
        port: args.port,
        addr: interface.link_local_addr,
        flags: args.flags,
    };
    let _registration = Mdns::new(interface.index).register(service).await?;
    eprintln!(
        "receiving as {display_name:?} ({service_id}) on {} [{}]:{}",
        interface.name, interface.link_local_addr, args.port
    );

    let mut stdin_lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    loop {
        let event = tokio::select! {
            event = events.recv() => event,
            _ = tokio::signal::ctrl_c() => return Ok(ExitCode::SUCCESS),
        };
        let Some(event) = event else {
            return Ok(ExitCode::SUCCESS);
        };
        match event {
            ReceiverEvent::Discovered { peer, .. } => eprintln!("seen by {peer}"),
            ReceiverEvent::Request(request) => {
                eprintln!(
                    "{:?} ({:?}) wants to send:",
                    request.ask.sender_computer_name, request.ask.sender_model_name
                );
                print_offer(&request.ask.offer);
                if args.accept {
                    request.accept();
                    continue;
                }
                eprint!("accept? [y/N] ");
                match tokio::time::timeout(ASK_TIMEOUT, stdin_lines.next_line()).await {
                    Ok(Ok(Some(line))) if line.trim().eq_ignore_ascii_case("y") => request.accept(),
                    Ok(_) => request.decline(),
                    Err(_) => eprintln!("\ntimed out"),
                }
            }
            ReceiverEvent::Withdrawn { .. } => eprintln!("the request was withdrawn"),
            ReceiverEvent::Urls { urls, .. } => urls.iter().for_each(|url| println!("{url:?}")),
            ReceiverEvent::Received { paths, .. } => {
                paths.iter().for_each(|path| println!("{}", path.display()))
            }
            ReceiverEvent::Failed { reason, .. } => eprintln!("transfer failed: {reason}"),
        }
    }
}

pub async fn browse(args: BrowseArgs) -> io::Result<ExitCode> {
    let (interface, display_name) = args.link.resolve()?;
    let sender = new_sender(display_name)?;
    browse_receivers(&interface, &sender, Duration::from_secs(args.seconds), |listed| {
        println!("{:<14} {:?} ({})", listed.instance, listed.name, listed.addr);
        false
    })
    .await?;
    Ok(ExitCode::SUCCESS)
}

pub async fn send(args: SendArgs) -> io::Result<ExitCode> {
    let (interface, display_name) = args.link.resolve()?;
    let payload = match (args.url.is_empty(), args.files.is_empty()) {
        (false, true) => Payload::Urls(args.url),
        (true, false) => Payload::Files(
            args.files
                .iter()
                .map(std::path::absolute)
                .collect::<Result<_, _>>()?,
        ),
        _ => return Err(io::Error::other("give either files or --url, not both")),
    };
    let sender = new_sender(display_name)?;
    let target = match args.addr {
        Some(ip) => {
            let addr = SocketAddrV6::new(ip, args.port, 0, interface.index);
            let receiver = sender
                .discover(addr)
                .await?
                .ok_or_else(|| io::Error::other("the receiver at that address doesn't list us"))?;
            ListedReceiver {
                instance: addr.to_string(),
                name: receiver.computer_name,
                addr,
                flags: args.flags,
            }
        }
        None => {
            match &args.to {
                Some(target_name) => {
                    eprintln!("looking for {target_name:?} for {} s...", args.seconds)
                }
                None => eprintln!("looking for a receiver for {} s...", args.seconds),
            }
            find_receiver(
                &interface,
                &sender,
                args.to.as_deref(),
                Duration::from_secs(args.seconds),
            )
            .await?
        }
    };
    eprintln!("sending to {:?} at {}", target.name, target.addr);

    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    let progress_printer = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                Event::Accepted => eprintln!("accepted"),
                Event::Progress(progress) => {
                    eprint!("\r{} / {} bytes", progress.sent, progress.total)
                }
            }
        }
        eprintln!();
    });
    let outcome = sender
        .send(target.addr, &payload, target.flags, Some(event_tx))
        .await?;
    let _ = progress_printer.await;
    Ok(match outcome {
        Outcome::Delivered => {
            println!("delivered");
            ExitCode::SUCCESS
        }
        Outcome::Declined => {
            println!("declined");
            ExitCode::from(1)
        }
        Outcome::Failed(reason) => {
            println!("failed: {reason}");
            ExitCode::from(2)
        }
    })
}

async fn find_receiver(
    interface: &LinkInterface,
    sender: &Sender,
    target_name: Option<&str>,
    limit: Duration,
) -> io::Result<ListedReceiver> {
    let mut chosen = None;
    browse_receivers(interface, sender, limit, |listed| {
        let wanted = target_name.is_none_or(|name| name == listed.name || name == listed.instance);
        if wanted {
            chosen = Some(listed.clone());
        }
        wanted
    })
    .await?;
    chosen.ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "no such receiver found"))
}

async fn browse_receivers(
    interface: &LinkInterface,
    sender: &Sender,
    limit: Duration,
    mut on_listed: impl FnMut(&ListedReceiver) -> bool,
) -> io::Result<()> {
    let mut browse = Mdns::new(interface.index).browse().await?;
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        let event = tokio::select! {
            event = browse.events.recv() => event,
            () = tokio::time::sleep_until(deadline) => return Ok(()),
        };
        let Some(BrowseEvent::Found(found)) = event else {
            continue;
        };
        let addr = SocketAddrV6::new(found.addr, found.port, 0, interface.index);
        match sender.discover(addr).await {
            Ok(Some(receiver)) => {
                let listed = ListedReceiver {
                    instance: found.instance,
                    name: receiver.computer_name,
                    addr,
                    flags: found.flags,
                };
                if on_listed(&listed) {
                    return Ok(());
                }
            }
            Ok(None) => eprintln!("{} doesn't list us", found.instance),
            Err(error) => eprintln!("{}: /Discover failed: {error}", found.instance),
        }
    }
}

fn print_offer(offer: &Offer) {
    match offer {
        Offer::Files(files) => {
            for file in files {
                let size_label = file
                    .size
                    .map(|size| format!("{size} bytes"))
                    .unwrap_or_default();
                eprintln!("  {:?} {} {size_label}", file.name, file.file_type);
            }
        }
        Offer::Urls(urls) => urls.iter().for_each(|url| eprintln!("  {url:?}")),
    }
}

fn new_sender(display_name: String) -> io::Result<Sender> {
    let mut config = sender::Config::new(display_name, random_service_id()?);
    config.model_name = platform_model_name();
    Sender::new(config)
}

fn platform_model_name() -> String {
    crosswind_core::Platform::current().display_name().into()
}

fn state_dir() -> io::Result<PathBuf> {
    dirs::data_local_dir()
        .map(|dir| dir.join("crosswind"))
        .ok_or_else(|| io::Error::other("no local data directory for this user"))
}

fn random_service_id() -> io::Result<String> {
    let mut bytes = [0; 6];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(mdns::service_id(bytes))
}

fn parse_flags(text: &str) -> Result<u32, String> {
    match text.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => text.parse(),
    }
    .map_err(|error| format!("{text}: {error}"))
}
