use crate::state::lan_ip;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use zeroconf::prelude::*;
use zeroconf::{MdnsService, ServiceType, TxtRecord};

pub struct MdnsGuard {
    thread: Option<JoinHandle<()>>,
}

impl MdnsGuard {
    fn start(port: u16) -> Option<Self> {
        let thread = thread::spawn(move || {
            let Some(service_type) = ServiceType::new(crate::protocol::SERVICE_NAME, "tcp").ok()
            else {
                tracing::warn!("mDNS service type failed");
                return;
            };
            let mut service = MdnsService::new(service_type, port);
            service.set_name(crate::protocol::PRODUCT_NAME);

            let mut txt = TxtRecord::new();
            if txt.insert("proto", "1").is_err() {
                tracing::warn!("mDNS txt record failed");
                return;
            }
            service.set_txt_record(txt);
            service.set_registered_callback(Box::new(|result, _| {
                if let Err(err) = result {
                    tracing::warn!("mDNS register failed: {err}");
                }
            }));

            let Ok(event_loop) = service.register() else {
                tracing::warn!("mDNS register failed");
                return;
            };

            loop {
                if event_loop.poll(Duration::from_millis(250)).is_err() {
                    break;
                }
            }
        });

        Some(Self {
            thread: Some(thread),
        })
    }
}

impl Drop for MdnsGuard {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn advertise(port: u16) -> Option<MdnsGuard> {
    let ip = lan_ip();
    let service = crate::protocol::SERVICE_NAME;
    let guard = MdnsGuard::start(port)?;
    tracing::info!(
        "mDNS advertised _{service}._tcp on {ip} ({})",
        crate::state::local_host_fqdn()
    );
    Some(guard)
}
