use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use proto::{backend::ResponseBackendMessage, frontend::RequestFrontendMessage};
use tokio::time::{self, MissedTickBehavior};

use super::BackendHandle;

const MAX_SAMPLES: usize = 20;

#[derive(Clone, Debug)]
pub struct Sample {
    pub cpu: f32,
    pub temp: Option<f32>,
    pub ram: f32,
    pub swap: f32,
    pub sent: f32,
    pub recv: f32,
}

#[derive(Clone, Debug, Default)]
pub struct History(Arc<Mutex<VecDeque<Sample>>>);

impl History {
    pub fn snapshot(&self) -> Vec<Sample> {
        self.0.lock().unwrap().iter().rev().cloned().collect()
    }

    fn push(&self, sample: Sample) {
        let mut samples = self.0.lock().unwrap();
        if samples.len() == MAX_SAMPLES {
            samples.pop_front();
        }
        samples.push_back(sample);
    }

    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

fn percent(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        ((used as f64 / total as f64) * 10000.0).round() as f32 / 100.0
    }
}

async fn collect(handle: &BackendHandle) -> anyhow::Result<Sample> {
    let cpu = handle.send_req(RequestFrontendMessage::Cpu).await?;
    let temp = handle.send_req(RequestFrontendMessage::Temp).await?;
    let mem = handle.send_req(RequestFrontendMessage::Mem).await?;
    let network = handle.send_req(RequestFrontendMessage::NetIO).await?;
    let (
        ResponseBackendMessage::Cpu(cpu),
        ResponseBackendMessage::Temp(temp),
        ResponseBackendMessage::Mem(mem),
        ResponseBackendMessage::NetIO(network),
    ) = (cpu, temp, mem, network)
    else {
        anyhow::bail!("unexpected monitoring response");
    };
    Ok(Sample {
        cpu: cpu.global_cpu,
        temp: temp.temp,
        ram: percent(mem.ram.used, mem.ram.total),
        swap: percent(mem.swap.used, mem.swap.total),
        sent: network.sent as f32,
        recv: network.recv as f32,
    })
}

pub async fn sample(handle: BackendHandle) {
    let mut interval = time::interval(Duration::from_secs(2));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if handle.is_closed() {
            break;
        }
        match time::timeout(Duration::from_secs(4), collect(&handle)).await {
            Ok(Ok(sample)) => handle.history.push(sample),
            _ => handle.history.clear(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{History, MAX_SAMPLES, Sample, percent};

    #[test]
    fn keeps_recent_samples_bounded_and_newest_first() {
        let history = History::default();
        for i in 0..MAX_SAMPLES + 3 {
            history.push(Sample {
                cpu: i as f32,
                temp: None,
                ram: 0.0,
                swap: 0.0,
                sent: 0.0,
                recv: 0.0,
            });
        }
        let result = history.snapshot();
        assert_eq!(result.len(), MAX_SAMPLES);
        assert_eq!(result[0].cpu, (MAX_SAMPLES + 2) as f32);
        assert_eq!(result[MAX_SAMPLES - 1].cpu, 3.0);
        history.clear();
        assert!(history.snapshot().is_empty());
    }

    #[test]
    fn zero_capacity_does_not_divide_by_zero() {
        assert_eq!(percent(0, 0), 0.0);
        assert_eq!(percent(1, 4), 25.0);
    }
}
