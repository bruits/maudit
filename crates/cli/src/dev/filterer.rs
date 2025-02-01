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
            if let Tag::Path { path, file_type: _ } = tag {
                if path.ancestors().any(|p| p.ends_with("dist")) {
                    result = false;
                    break;
                }
            }

            if let Tag::FileEventKind(FileEventKind::Modify(ModifyKind::Metadata(_))) = tag {
                result = false;
                break;
            }
        }

        if result {
            println!("{:?}", event);
        }

        Ok(result)
    }
}
