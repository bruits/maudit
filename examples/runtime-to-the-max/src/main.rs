use maudit::{BuildOptions, BuildOutput, content_sources, coronate, route::prelude::*};

#[route(format!("/dynamic/{}/", self.dynamic_page.0))]
struct Dynamic {
    dynamic_page: (String, String),
}

impl Route for Dynamic {
    fn render(&self, _: &mut PageContext) -> impl Into<RenderResult> {
        self.dynamic_page.1.clone()
    }
}

fn main() -> Result<BuildOutput, Box<dyn std::error::Error>> {
    let routes: Vec<Box<dyn FullRoute>> = std::fs::read_to_string("pages.txt")?
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ": ");
            match (parts.next(), parts.next()) {
                (Some(name), Some(content)) => Some((name.to_string(), content.to_string())),
                _ => None,
            }
        })
        .map(|dynamic_page| Box::new(Dynamic { dynamic_page }) as Box<dyn FullRoute>)
        .collect();

    coronate(
        &routes.iter().map(|r| r.as_ref()).collect::<Vec<_>>(),
        content_sources![],
        BuildOptions::default(),
    )
}
