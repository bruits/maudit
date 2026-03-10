use crate::content::{ContentSource, Entry};
use crate::route::{Page, PageParams, Pages};
use std::cell::RefCell;
use std::rc::Rc;

/// Records content access patterns during page rendering.
///
/// After a page is rendered, the access log is used to determine
/// the page's content dependencies for incremental build caching.
#[derive(Debug, Clone, Default)]
pub struct ContentAccessLog {
    /// Entries accessed by get_entry() — (source_name, entry_id).
    pub entries_read: Vec<(String, String)>,
    /// Sources fully iterated (via into_pages, into_params, or entries()).
    pub sources_iterated: Vec<String>,
}

impl ContentAccessLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge specific entry reads from another access log into this one.
    /// Used to include content dependencies from `get_pages()` into each
    /// individual page's log. Only merges `entries_read` (not `sources_iterated`),
    /// because source iteration during `get_pages()` is for page enumeration —
    /// structural changes to iterated sources already trigger a full rebuild
    /// of the dynamic route via `find_stale_pages`.
    pub fn merge_entries_read(&mut self, other: &ContentAccessLog) {
        self.entries_read.extend(other.entries_read.iter().cloned());
    }
}

/// A wrapper around [`ContentSource`] that records all accesses
/// for incremental build dependency tracking.
///
/// Obtained via [`PageContext::content()`](crate::route::PageContext::content)
/// or [`DynamicRouteContext::content()`](crate::route::DynamicRouteContext::content).
pub struct TrackedContentSource<'a, T> {
    pub(crate) inner: &'a ContentSource<T>,
    pub(crate) source_name: String,
    pub(crate) log: Rc<RefCell<ContentAccessLog>>,
}

impl<'a, T> TrackedContentSource<'a, T> {
    /// Get a single entry by ID. Records a dependency on this specific entry.
    pub fn get_entry(&self, id: &str) -> &'a Entry<T> {
        self.log
            .borrow_mut()
            .entries_read
            .push((self.source_name.clone(), id.to_string()));
        self.inner.get_entry(id)
    }

    /// Get a single entry by ID, returning None if not found.
    /// Records a dependency on this specific entry.
    pub fn get_entry_safe(&self, id: &str) -> Option<&'a Entry<T>> {
        self.log
            .borrow_mut()
            .entries_read
            .push((self.source_name.clone(), id.to_string()));
        self.inner.get_entry_safe(id)
    }

    /// Access all entries. Marks this source as fully iterated —
    /// any change to any entry in this source will trigger a rebuild.
    pub fn entries(&self) -> impl Iterator<Item = &'a Entry<T>> {
        self.log
            .borrow_mut()
            .sources_iterated
            .push(self.source_name.clone());
        self.inner.entries.values()
    }

    /// Convert entries to pages. Marks this source as fully iterated.
    /// Also records which entry produced each page, so that per-page
    /// content dependencies can be tracked even when `build()` only uses props.
    pub fn into_pages<Params, Props>(
        &self,
        mut cb: impl FnMut(&Entry<T>) -> Page<Params, Props>,
    ) -> Pages<Params, Props>
    where
        Params: Into<PageParams>,
    {
        self.log
            .borrow_mut()
            .sources_iterated
            .push(self.source_name.clone());
        let source_name = self.source_name.clone();
        self.inner.into_pages(|entry| {
            let mut page = cb(entry);
            page._source_entry = Some((source_name.clone(), entry.id.clone()));
            page
        })
    }

    /// Convert entries to params. Marks this source as fully iterated.
    pub fn into_params<P>(&self, cb: impl FnMut(&Entry<T>) -> P) -> Vec<P>
    where
        P: Into<PageParams>,
    {
        self.log
            .borrow_mut()
            .sources_iterated
            .push(self.source_name.clone());
        self.inner.into_params(cb)
    }

    /// Get the name of the underlying content source.
    pub fn name(&self) -> &str {
        &self.inner.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ContentEntry, ContentSource, ContentSourceInternal};

    fn make_test_source() -> ContentSource<String> {
        let mut source = ContentSource::new(
            "test_source",
            Box::new(|| {
                vec![
                    Entry::<String>::create(
                        "entry1".to_string(),
                        None,
                        None,
                        "data1".to_string(),
                        vec![],
                    ),
                    Entry::<String>::create(
                        "entry2".to_string(),
                        None,
                        None,
                        "data2".to_string(),
                        vec![],
                    ),
                ]
            }),
        );
        source.init();
        source
    }

    #[test]
    fn test_get_entry_records_access() {
        let source = make_test_source();
        let log = Rc::new(RefCell::new(ContentAccessLog::new()));
        let tracked = TrackedContentSource {
            inner: &source,
            source_name: "test_source".to_string(),
            log: log.clone(),
        };

        let _ = tracked.get_entry("entry1");

        let access_log = log.borrow();
        assert_eq!(access_log.entries_read.len(), 1);
        assert_eq!(
            access_log.entries_read[0],
            ("test_source".to_string(), "entry1".to_string())
        );
        assert!(access_log.sources_iterated.is_empty());
    }

    #[test]
    fn test_entries_records_full_iteration() {
        let source = make_test_source();
        let log = Rc::new(RefCell::new(ContentAccessLog::new()));
        let tracked = TrackedContentSource {
            inner: &source,
            source_name: "test_source".to_string(),
            log: log.clone(),
        };

        let _ = tracked.entries();

        let access_log = log.borrow();
        assert!(access_log.entries_read.is_empty());
        assert_eq!(access_log.sources_iterated.len(), 1);
        assert_eq!(access_log.sources_iterated[0], "test_source");
    }
}
