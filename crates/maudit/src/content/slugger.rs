use rustc_hash::FxHashSet;
use slug::slugify;

pub struct Slugger {
    generated_slugs: FxHashSet<String>,
}

impl Slugger {
    pub fn new() -> Self {
        Self {
            generated_slugs: FxHashSet::default(),
        }
    }

    pub fn slugify(&mut self, text: &str) -> String {
        let mut slug = slugify(text);
        let mut counter = 1;
        while self.generated_slugs.contains(&slug) {
            slug = format!("{}-{}", slug, counter);
            counter += 1;
        }
        self.generated_slugs.insert(slug.clone());
        slug
    }
}
