//! Build-scoped derived values: data computed once per build from content and shared
//! across every page that reads it.
//!
//! A [`BuildValue`] replaces the fragile pattern of one route computing something during
//! its `render()` and stashing it in a global for other routes to read. On incremental
//! builds the producing route can be served from cache — skipping its `render()` — which
//! leaves the global unset and breaks its readers. A build value is instead computed
//! lazily *on read*, memoized for the rest of the build, and — crucially — records its own
//! content dependencies onto every page that reads it, so those pages are re-rendered
//! exactly when the value's inputs change.
//!
//! ## Example
//! ```rust
//! use maudit::route::prelude::*;
//! use maudit::build_value::BuildValue;
//! # use maudit::content::markdown_entry;
//! # #[markdown_entry]
//! # pub struct Article { pub title: String }
//!
//! static ARTICLE_COUNT: BuildValue<usize> = BuildValue::new(|ctx| {
//!     ctx.content::<Article>("articles").entries().count()
//! });
//!
//! #[route("/")]
//! pub struct Index;
//! impl Route for Index {
//!     fn render(&self, ctx: &mut PageContext) -> impl Into<RenderResult> {
//!         format!("{} articles", ctx.build_value(&ARTICLE_COUNT))
//!     }
//! }
//! ```

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::content::tracked::{ContentAccessLog, TrackedContentSource};
use crate::content::{ContentContext, ContentSources};

/// A value derived once per build from content, read by any number of pages.
///
/// Declare it as a `static` with a non-capturing closure that reads everything it needs
/// from its [`DerivedContext`], then read it from a route with
/// [`PageContext::build_value`](crate::route::PageContext::build_value). The closure runs
/// at most once per build (the first time any page reads it) and its result is shared.
pub struct BuildValue<T> {
    compute: fn(&mut DerivedContext) -> T,
}

impl<T> BuildValue<T> {
    /// Create a build value from a pure computation over content. The closure must not
    /// capture state (so it can live in a `static`); read inputs from the context instead.
    pub const fn new(compute: fn(&mut DerivedContext) -> T) -> Self {
        Self { compute }
    }
}

/// Per-build memoization store for [`BuildValue`]s. Created once at the start of a build
/// and shared (by reference) with every [`PageContext`](crate::route::PageContext).
///
/// [`coronate`](crate::coronate) manages one automatically; you only construct this
/// directly when writing a custom build loop.
#[derive(Default)]
pub struct BuildValueStore {
    cache: RefCell<FxHashMap<usize, CachedBuildValue>>,
}

struct CachedBuildValue {
    value: Rc<dyn Any>,
    deps: ContentAccessLog,
}

/// Context handed to a [`BuildValue`]'s compute closure. Exposes tracked content access
/// (and nested build values), but deliberately no asset access — a build value is pure
/// derived data; anything that emits assets belongs to a route's `render()`.
pub struct DerivedContext<'a> {
    content: &'a ContentSources,
    access_log: Rc<RefCell<ContentAccessLog>>,
    store: &'a BuildValueStore,
}

impl<'a> DerivedContext<'a> {
    /// Get a tracked content source by name. Reads are recorded as the build value's
    /// dependencies and replayed onto every page that reads the value.
    pub fn content<T: 'static>(&self, name: &str) -> TrackedContentSource<'a, T> {
        TrackedContentSource {
            inner: self.content.get_source::<T>(name),
            source_name: name.to_string(),
            log: self.access_log.clone(),
        }
    }

    /// Read another build value from within this one. Its dependencies fold into this
    /// value's dependencies.
    pub fn build_value<T: 'static>(&self, def: &'static BuildValue<T>) -> Rc<T> {
        compute(def, self.content, self.store, &self.access_log)
    }
}

impl ContentContext for DerivedContext<'_> {
    fn content(&self) -> &ContentSources {
        self.content
    }
}

/// Resolve a build value: return the memoized result if present, otherwise compute it
/// with a fresh dependency log. Either way, replay the value's dependencies onto
/// `reader_log` so the reading page depends on whatever the value was computed from.
pub(crate) fn compute<T: 'static>(
    def: &'static BuildValue<T>,
    content: &ContentSources,
    store: &BuildValueStore,
    reader_log: &Rc<RefCell<ContentAccessLog>>,
) -> Rc<T> {
    let key = def as *const BuildValue<T> as usize;

    {
        let cache = store.cache.borrow();
        if let Some(cached) = cache.get(&key) {
            reader_log.borrow_mut().merge_all(&cached.deps);
            return cached
                .value
                .clone()
                .downcast::<T>()
                .expect("build value cached under a mismatched type");
        }
    }

    // Compute with a fresh log so the closure's reads become *this value's* deps rather
    // than the current reader's. Nested build_value() calls fold their deps in here too.
    let fresh_log = Rc::new(RefCell::new(ContentAccessLog::new()));
    let mut derived = DerivedContext {
        content,
        access_log: fresh_log.clone(),
        store,
    };
    let value: Rc<T> = Rc::new((def.compute)(&mut derived));
    let deps = fresh_log.take();

    reader_log.borrow_mut().merge_all(&deps);
    store.cache.borrow_mut().insert(
        key,
        CachedBuildValue {
            value: value.clone(),
            deps,
        },
    );
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ContentEntry, ContentSource, ContentSources, Entry};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn sources() -> ContentSources {
        let source = ContentSource::<String>::new(
            "things",
            Box::new(|| {
                vec![
                    Entry::<String>::create("a".into(), None, None, "one".into(), vec![]),
                    Entry::<String>::create("b".into(), None, None, "two".into(), vec![]),
                ]
            }),
        );
        let mut sources = ContentSources::new(vec![Box::new(source)]);
        sources.init_all();
        sources
    }

    static COMPUTE_COUNT: AtomicUsize = AtomicUsize::new(0);
    static THING_COUNT: BuildValue<usize> = BuildValue::new(|ctx| {
        COMPUTE_COUNT.fetch_add(1, Ordering::SeqCst);
        ctx.content::<String>("things").entries().count()
    });

    #[test]
    fn computes_once_and_replays_deps() {
        let sources = sources();
        let store = BuildValueStore::default();
        let reader = Rc::new(RefCell::new(ContentAccessLog::new()));

        COMPUTE_COUNT.store(0, Ordering::SeqCst);
        let first = compute(&THING_COUNT, &sources, &store, &reader);
        let second = compute(&THING_COUNT, &sources, &store, &reader);

        assert_eq!(*first, 2);
        assert_eq!(*second, 2);
        // Memoized: the closure ran exactly once across both reads.
        assert_eq!(COMPUTE_COUNT.load(Ordering::SeqCst), 1);

        // The value iterated "things", so each reader now depends on that source.
        let log = reader.borrow();
        assert_eq!(
            log.sources_iterated
                .iter()
                .filter(|s| s.as_str() == "things")
                .count(),
            2,
            "the source dep should be replayed onto every read"
        );
    }

    // Separate build values from THING_COUNT so this test never touches COMPUTE_COUNT
    // (unit tests run in parallel; a shared counter would race).
    static INNER_COUNT: BuildValue<usize> =
        BuildValue::new(|ctx| ctx.content::<String>("things").entries().count());
    static DERIVED_LEN: BuildValue<usize> =
        BuildValue::new(|ctx| *ctx.build_value(&INNER_COUNT) + 10);

    #[test]
    fn nested_build_value_folds_in_deps() {
        let sources = sources();
        let store = BuildValueStore::default();
        let reader = Rc::new(RefCell::new(ContentAccessLog::new()));

        let value = compute(&DERIVED_LEN, &sources, &store, &reader);
        assert_eq!(*value, 12);

        // The nested value iterated "things", so the outer reader inherits that dep.
        assert!(
            reader
                .borrow()
                .sources_iterated
                .iter()
                .any(|s| s == "things")
        );
    }
}
