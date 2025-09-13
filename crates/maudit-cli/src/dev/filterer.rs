use watchexec::filter::Filterer;
use watchexec_events::{
    filekind::{FileEventKind, ModifyKind},
    Tag,
};

#[derive(Debug)]
pub struct DevServerFilterer;

impl Filterer for DevServerFilterer {
    fn check_event(
        &self,
        event: &watchexec_events::Event,
        _: watchexec_events::Priority,
    ) -> Result<bool, watchexec::error::RuntimeError> {
        let mut result = true;

        for tag in &event.tags {
            // NOTE: This happens whenever the watch gets dropped and re-added, you get something like `rescan: user dropped`
            // It's probable that this needs to be used to do some sort of action on the watch, not sure what yet
            if let Tag::FileEventKind(FileEventKind::Other) = tag {
                result = false;
                break;
            }

            if let Tag::Path { path, file_type: _ } = tag {
                if let Some(file_name) = path.file_name() {
                    if file_name == ".DS_Store" {
                        result = false;
                        break;
                    }
                }

                // TODO: Customizable dist path
                if path.ancestors().any(|p| p.ends_with("dist")) {
                    result = false;
                    break;
                }

                if path.ancestors().any(|p| p.ends_with("target")) {
                    result = false;
                    break;
                }
            }

            if let Tag::FileEventKind(FileEventKind::Modify(ModifyKind::Metadata(_))) = tag {
                result = false;
                break;
            }
        }

        Ok(result)
    }
}
