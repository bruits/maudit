use std::{path::PathBuf, sync::Arc, time::Instant};

use log::info;
use rolldown::{
    ModuleType,
    plugin::{HookUsage, Plugin},
};

use super::node_sidecar::NodeSidecar;

/// Rolldown plugin to process select CSS files with Tailwind CSS via the Node sidecar.
#[derive(Debug)]
pub struct TailwindPlugin {
    pub tailwind_entries: Vec<PathBuf>,
    pub sidecar: Arc<NodeSidecar>,
}

impl TailwindPlugin {
    /// Create a new TailwindPlugin with a reference to the Node sidecar
    pub fn new(tailwind_entries: Vec<PathBuf>, sidecar: Arc<NodeSidecar>) -> Self {
        Self {
            tailwind_entries,
            sidecar,
        }
    }
}

impl Plugin for TailwindPlugin {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "builtin:tailwind".into()
    }

    fn register_hook_usage(&self) -> rolldown::plugin::HookUsage {
        HookUsage::Transform
    }

    async fn transform(
        &self,
        _ctx: rolldown::plugin::SharedTransformPluginContext,
        args: &rolldown::plugin::HookTransformArgs<'_>,
    ) -> rolldown::plugin::HookTransformReturn {
        if *args.module_type != ModuleType::Css {
            return Ok(None);
        }

        if self
            .tailwind_entries
            .iter()
            .any(|entry| entry.canonicalize().unwrap().to_string_lossy() == args.id)
        {
            let start_tailwind = Instant::now();

            info!("Using Tailwind processor for {}", args.id);

            let minify = !crate::is_dev();
            let output = self
                .sidecar
                .process_tailwind(args.id, minify)
                .unwrap_or_else(|e| panic!("Tailwind processor failed: {}", e));

            info!("Tailwind took {:?}", start_tailwind.elapsed());

            return Ok(Some(rolldown::plugin::HookTransformOutput {
                code: Some(output),
                ..Default::default()
            }));
        }

        Ok(None)
    }
}
