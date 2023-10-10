use anyhow::Result;
use rand::Rng;
use std::{num::NonZeroUsize, time::Duration};
use tokio::sync::RwLock;

pub struct RLimit {
  count_tasks: RwLock<usize>,
  pub limit_per_second: NonZeroUsize,
}

impl RLimit {
  pub fn new(limit_per_second: NonZeroUsize) -> Result<Self> {
    Ok(Self {
      count_tasks: RwLock::new(0),
      limit_per_second,
    })
  }

  fn get_jittered_time() -> usize {
    rand::thread_rng().gen_range(1_usize..100)
  }

  pub async fn wait(&self) {
    loop {
      if let Ok(count_tasks) = self.count_tasks.try_read() {
        if *count_tasks < self.limit_per_second.get() {
          break;
        }
      }

      tokio::time::sleep(Duration::from_millis(Self::get_jittered_time() as u64)).await;
    }

    loop {
      if let Ok(mut count_tasks) = self.count_tasks.try_write() {
        *count_tasks += 1;
        break;
      }

      tokio::time::sleep(Duration::from_millis(Self::get_jittered_time() as u64)).await;
    }
  }

  pub async fn done(&self) -> Result<()> {
    let mut count_tasks = self.count_tasks.write().await;

    if *count_tasks > 0 {
      *count_tasks -= 1;
      Ok(())
    } else {
      Err(anyhow::anyhow!("called done too much"))
    }
  }
}
