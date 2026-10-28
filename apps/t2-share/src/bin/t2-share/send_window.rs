// SPDX-License-Identifier: GPL-3.0-or-later
//! Send window: looks for phones, sends the chosen files to the one the
//! user picks and shows the PIN and the progress.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

use adw::prelude::*;
use gtk4 as gtk;
use tokio::task::JoinHandle;

use t2_share::discover::{Discovery, Peer};
use t2_share::endpoint::DeviceType;
use t2_share::send::{self, Progress, SendOutcome};

/// The async work runs on a tokio runtime next to the GTK main loop.
fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

enum Event {
    Found(Peer),
    DiscoveryFailed(String),
    Waiting(String),
    Sent(u64, u64),
    Done(Result<SendOutcome, String>),
}

struct ChannelProgress(async_channel::Sender<Event>);

impl Progress for ChannelProgress {
    fn waiting(&mut self, pin: &str) {
        let _ = self.0.try_send(Event::Waiting(pin.to_string()));
    }

    fn sent(&mut self, sent: u64, total: u64) {
        let _ = self.0.try_send(Event::Sent(sent, total));
    }
}

pub fn open(app: &adw::Application, files: Vec<PathBuf>) {
    let files: Vec<PathBuf> = files.into_iter().filter(|f| f.is_file()).collect();
    if files.is_empty() {
        return;
    }

    let status = adw::StatusPage::builder()
        .icon_name("network-wireless-symbolic")
        .title("Looking for phones")
        .description(
            "The phone has to be in the same Wi-Fi, with Bluetooth on and Quick Share \
             visible to everyone.",
        )
        .vexpand(true)
        .build();
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(12)
        .build();
    let progress = gtk::ProgressBar::builder()
        .margin_start(24)
        .margin_end(24)
        .margin_bottom(24)
        .visible(false)
        .build();
    let summary = gtk::Label::builder()
        .label(describe(&files))
        .wrap(true)
        .css_classes(["dim-label"])
        .margin_top(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.append(&summary);
    body.append(&status);
    body.append(&list);
    body.append(&progress);

    let header = adw::HeaderBar::new();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&body));
    let window = adw::Window::builder()
        .application(app)
        .title("Send with T2 Share")
        .default_width(420)
        .default_height(520)
        .content(&view)
        .build();

    let (tx, rx) = async_channel::unbounded::<Event>();
    let tasks: Rc<RefCell<Vec<JoinHandle<()>>>> = Rc::default();
    {
        let tx = tx.clone();
        tasks.borrow_mut().push(runtime().spawn(async move {
            let mut discovery = match Discovery::start().await {
                Ok(discovery) => discovery,
                Err(err) => {
                    let _ = tx.send(Event::DiscoveryFailed(format!("{err:#}"))).await;
                    return;
                }
            };
            loop {
                match discovery.next().await {
                    Ok(peer) => {
                        if tx.send(Event::Found(peer)).await.is_err() {
                            return;
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(Event::DiscoveryFailed(format!("{err:#}"))).await;
                        return;
                    }
                }
            }
        }));
    }
    {
        let tasks = tasks.clone();
        window.connect_close_request(move |_| {
            for task in tasks.borrow_mut().drain(..) {
                task.abort();
            }
            glib::Propagation::Proceed
        });
    }

    let files = Rc::new(files);
    glib::spawn_future_local(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                Event::Found(peer) => {
                    let row = adw::ActionRow::builder()
                        .title(peer.label())
                        .subtitle(peer.addr.ip().to_string())
                        .activatable(true)
                        .build();
                    row.add_prefix(&gtk::Image::from_icon_name(icon(peer.device_type)));
                    let tx = tx.clone();
                    let tasks = tasks.clone();
                    let files = files.clone();
                    let row_status = status.clone();
                    let row_list = list.clone();
                    row.connect_activated(move |_| {
                        // Stop looking, the BLE advertisement ends with it.
                        for task in tasks.borrow_mut().drain(..) {
                            task.abort();
                        }
                        row_list.set_sensitive(false);
                        row_status.set_title(&format!("Connecting to {}", peer.label()));
                        row_status.set_description(None);
                        let tx = tx.clone();
                        let files = files.as_ref().clone();
                        let addr = peer.addr;
                        tasks.borrow_mut().push(runtime().spawn(async move {
                            let mut progress = ChannelProgress(tx.clone());
                            let result = send::send(addr, &local_name(), &files, &mut progress)
                                .await
                                .map_err(|err| format!("{err:#}"));
                            let _ = tx.send(Event::Done(result)).await;
                        }));
                    });
                    list.append(&row);
                    status.set_title("Choose a phone");
                }
                Event::DiscoveryFailed(err) => {
                    status.set_icon_name(Some("dialog-warning-symbolic"));
                    status.set_title("Cannot look for phones");
                    status.set_description(Some(&err));
                }
                Event::Waiting(pin) => {
                    status.set_title("Accept on the phone");
                    status.set_description(Some(&format!("PIN {pin}")));
                    progress.set_visible(true);
                    progress.set_fraction(0.0);
                }
                Event::Sent(sent, total) => {
                    status.set_title("Sending");
                    progress.set_fraction(sent as f64 / total.max(1) as f64);
                }
                Event::Done(result) => {
                    progress.set_visible(false);
                    match result {
                        Ok(SendOutcome::Sent) => {
                            status.set_icon_name(Some("emblem-ok-symbolic"));
                            status.set_title("Sent");
                            status.set_description(None);
                        }
                        Ok(SendOutcome::Declined(_)) => {
                            status.set_icon_name(Some("action-unavailable-symbolic"));
                            status.set_title("Declined on the phone");
                            status.set_description(None);
                        }
                        Err(err) => {
                            status.set_icon_name(Some("dialog-warning-symbolic"));
                            status.set_title("Sending failed");
                            status.set_description(Some(&err));
                        }
                    }
                    list.set_visible(false);
                }
            }
        }
    });

    window.present();
}

fn describe(files: &[PathBuf]) -> String {
    let names: Vec<String> = files
        .iter()
        .take(3)
        .filter_map(|f| f.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .collect();
    let mut text = names.join(", ");
    if files.len() > 3 {
        text.push_str(&format!(" and {} more", files.len() - 3));
    }
    text
}

fn icon(device_type: DeviceType) -> &'static str {
    match device_type {
        DeviceType::Phone => "phone-symbolic",
        DeviceType::Tablet => "tablet-symbolic",
        DeviceType::Laptop => "computer-symbolic",
        DeviceType::Unknown => "network-wireless-symbolic",
    }
}

fn local_name() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|h| h.trim().to_string())
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "Linux".to_string())
}
