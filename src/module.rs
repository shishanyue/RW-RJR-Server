use std::sync::{Arc, RwLock};

use tokio::runtime::{Builder, Runtime};

use self::rw_engine::RwEngine;

mod rw_engine;
mod easy_dummy;

lazy_static! {
    static ref MODULE_RUNTIME:Runtime = Builder::new_multi_thread()
    .enable_time()
    .worker_threads(10)
    // no timer!
    .build()
    .expect("creat block runtime error");
    pub static ref MODULE_MANAGER: Arc<RwLock<ModuleManager>> = Arc::new(RwLock::new(ModuleManager::new()));
}

pub trait Module:Sync+Send {}

pub enum ModuleType {
    RwEngine,
}
pub struct ModuleManager {
    module_list: Vec<Box<dyn Module>>,
}

impl ModuleManager {
    pub fn new() -> Self {
        ModuleManager {
            module_list: Vec::new(),
        }
    }

    pub fn init_module(&mut self, module_type: ModuleType) {
        match module_type {
            ModuleType::RwEngine => self.module_list.push(Box::new(RwEngine::new())),
        }
    }
}
