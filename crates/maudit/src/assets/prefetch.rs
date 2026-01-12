use rolldown::{
    ModuleType,
    plugin::{HookUsage, Plugin},
};

const PREFETCH_CODE: &str = include_str!("../../js/prefetch.ts");

/// Rolldown plugin to expose the prefetch module as a virtual module.
#[derive(Debug)]
pub struct PrefetchPlugin;

impl Plugin for PrefetchPlugin {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "builtin:prefetch".into()
    }

    fn register_hook_usage(&self) -> HookUsage {
        HookUsage::ResolveId | HookUsage::Load
    }

    async fn resolve_id(
        &self,
        _ctx: &rolldown::plugin::PluginContext,
        args: &rolldown::plugin::HookResolveIdArgs<'_>,
    ) -> rolldown::plugin::HookResolveIdReturn {
        if args.specifier == "maudit:prefetch" {
            return Ok(Some(rolldown::plugin::HookResolveIdOutput {
                id: "maudit:prefetch".to_string().into(),
                ..Default::default()
            }));
        }
        Ok(None)
    }

    async fn load(
        &self,
        _ctx: &rolldown::plugin::PluginContext,
        args: &rolldown::plugin::HookLoadArgs<'_>,
    ) -> rolldown::plugin::HookLoadReturn {
        if args.id == "maudit:prefetch" {
            return Ok(Some(rolldown::plugin::HookLoadOutput {
                code: PREFETCH_CODE.to_string().into(),
                module_type: Some(ModuleType::Ts),
                ..Default::default()
            }));
        }
        Ok(None)
    }
}
