use zenith_core::{Engine, error::Result};
use std::sync::Arc;
use std::path::{PathBuf};
use notify::{Watcher, RecursiveMode, RecommendedWatcher, EventKind};
use tracing::{info, error, warn};
use std::fs;
use std::time::Duration;
use tokio::sync::broadcast;

// Runtime submodules
pub mod sandbox;
pub mod scheduler;
pub mod vm;
pub mod engine;
pub mod host_calls;

// Re-exports
pub use engine::{RuntimeEngine, PluginMetadata};
pub use sandbox::{Sandbox, SandboxLimits};
pub use scheduler::{Scheduler, Priority};
pub use vm::VM;
pub use host_calls::HostCallInterface;

/// The Zenith Runtime Manager.
/// Handles lifecycle, configuration, and hot-reloading of plugins.
pub struct Runtime {
    engine: Arc<Engine>,
    plugin_dir: PathBuf,
    shutdown_tx: broadcast::Sender<()>,
}

impl Runtime {
    /// Create a new Runtime environment
    pub fn new(buffer_size: usize, plugin_dir: impl Into<PathBuf>) -> Result<Self> {
        let engine = Arc::new(Engine::new(buffer_size)?);
        let path = plugin_dir.into();
        
        let (tx, _) = broadcast::channel(1);
        
        Ok(Self {
            engine,
            plugin_dir: path,
            shutdown_tx: tx,
        })
    }

    /// initialize and start the runtime
    /// This enables the hot-reload watcher on the plugin directory.
    pub async fn run(&self) -> anyhow::Result<()> {
        info!("Initializing Zenith Runtime...");
        
        // 1. Initial Load of Plugins
        self.load_all_plugins()?;

        // 2. Start Engine Consumer
        self.engine.start();
        info!("Core Engine Started.");

        // 3. Start Hot-Reload Watcher
        let watcher_plugin_dir = self.plugin_dir.clone();
        let engine_ref = self.engine.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // Spawn watcher task
        tokio::spawn(async move {
            info!("Starting Hot-Reload Watcher on {:?}", watcher_plugin_dir);
            
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match RecommendedWatcher::new(tx, notify::Config::default()) {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to create file watcher: {}", e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(&watcher_plugin_dir, RecursiveMode::NonRecursive) {
                error!("Failed to watch plugin dir: {}", e);
                return;
            }

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Watcher shutting down.");
                        break;
                    }
                    // We need to poll the std channel. Since this is async context, 
                    // a blocking recv is not ideal, but for MVP watcher it's acceptable 
                    // if we wrap it or just use a small timeout loop.
                    // Better approach for simple MVP: check channel periodically or use blocking task.
                    // We'll use a simple loop with yield.
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {
                        while let Ok(res) = rx.try_recv() {
                            match res {
                                Ok(event) => {
                                    if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                                        for path in event.paths {
                                            if path.extension().is_some_and(|ext| ext == "wasm") {
                                                info!("Change detected in {:?}. Reloading...", path);
                                                if let Ok(bytes) = fs::read(&path) {
                                                    if let Err(e) = engine_ref.load_plugin(&bytes) {
                                                        error!("Failed to hot-reload plugin: {}", e);
                                                    } else {
                                                        info!("Plugin hot-reloaded successfully.");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                Err(e) => warn!("Watch error: {}", e),
                            }
                        }
                    }
                }
            }
        });

        // 4. Wait for shutdown signal (Ctrl+C)
        tokio::signal::ctrl_c().await?;
        info!("Shutdown signal received.");
        
        let _ = self.shutdown_tx.send(());
        self.engine.shutdown();
        info!("Zenith Runtime Shutdown Complete.");
        
        Ok(())
    }

    fn load_all_plugins(&self) -> anyhow::Result<()> {
        if !self.plugin_dir.exists() {
            fs::create_dir_all(&self.plugin_dir)?;
        }

        let entries = fs::read_dir(&self.plugin_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "wasm") {
                info!("Loading plugin: {:?}", path);
                let bytes = fs::read(&path)?;
                self.engine.load_plugin(&bytes)?;
            }
        }
        Ok(())
    }
}
