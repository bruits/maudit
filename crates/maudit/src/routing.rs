use std::path::Path;

#[derive(Debug, PartialEq, Clone)]
pub struct ParameterDef {
    pub(crate) key: String,
    pub(crate) index: usize,
    pub(crate) length: usize,
}

pub fn extract_params_from_raw_route(raw_route: &str) -> Vec<ParameterDef> {
    let mut params = Vec::new();
    let mut start = 0;

    while let Some(bracket_pos) = raw_route[start..].find('[') {
        let abs_pos = start + bracket_pos;

        // Check if escaped by counting preceding backslashes
        let backslash_count = raw_route[..abs_pos]
            .chars()
            .rev()
            .take_while(|&c| c == '\\')
            .count();

        if backslash_count % 2 == 1 {
            start = abs_pos + 1;
            continue;
        }

        if let Some(end_bracket) = raw_route[abs_pos + 1..].find(']') {
            let end_pos = abs_pos + 1 + end_bracket;
            let key = raw_route[abs_pos + 1..end_pos].to_string();

            params.push(ParameterDef {
                key,
                index: abs_pos,
                length: end_pos - abs_pos + 1,
            });

            start = end_pos + 1;
        } else {
            break;
        }
    }

    // Sort by index in reverse order to avoid index shifting issues during replacement
    params.sort_by(|a, b| b.index.cmp(&a.index));

    params
}

pub fn guess_if_route_is_endpoint(raw_route: &str) -> bool {
    let real_path = Path::new(&raw_route);

    real_path.extension().is_some()
}

#[cfg(test)]
mod tests {
    use crate::routing::{ParameterDef, extract_params_from_raw_route, guess_if_route_is_endpoint};

    #[test]
    fn test_extract_params() {
        let input = "/articles/[article]";
        let expected = vec![ParameterDef {
            key: "article".to_string(),
            index: 10,
            length: 9,
        }];

        assert_eq!(extract_params_from_raw_route(input), expected);
    }

    #[test]
    fn test_extract_params_multiple() {
        let input = "/articles/[article]/[id]";
        let expected = vec![
            ParameterDef {
                key: "id".to_string(),
                index: 20,
                length: 4,
            },
            ParameterDef {
                key: "article".to_string(),
                index: 10,
                length: 9,
            },
        ];

        assert_eq!(extract_params_from_raw_route(input), expected);
    }

    #[test]
    fn test_extract_params_no_params() {
        let input = "/articles";
        let expected: Vec<ParameterDef> = Vec::new();

        assert_eq!(extract_params_from_raw_route(input), expected);
    }

    #[test]
    fn test_extract_params_escaped() {
        let input = "/articles/\\[article\\]";
        let expected: Vec<ParameterDef> = Vec::new();

        assert_eq!(extract_params_from_raw_route(input), expected);
    }

    #[test]
    fn test_extract_params_escaped_brackets() {
        let input = "/articles/\\[article\\]/\\[id\\]";
        let expected: Vec<ParameterDef> = Vec::new();

        assert_eq!(extract_params_from_raw_route(input), expected);
    }

    #[test]
    fn test_extract_params_escaped_brackets_with_params() {
        let input = "/articles/\\[article\\]/[id]";
        let expected = vec![ParameterDef {
            key: "id".to_string(),
            index: 22,
            length: 4,
        }];

        assert_eq!(extract_params_from_raw_route(input), expected);
    }

    #[test]
    fn test_guess_if_route_is_endpoint() {
        // Routes with file extensions should be detected as endpoints
        assert!(guess_if_route_is_endpoint("/api/data.json"));
        assert!(guess_if_route_is_endpoint("/feed.xml"));
        assert!(guess_if_route_is_endpoint("/sitemap.xml"));
        assert!(guess_if_route_is_endpoint("/robots.txt"));
        assert!(guess_if_route_is_endpoint("/path/to/file.tar.gz"));
        assert!(guess_if_route_is_endpoint("/api/users/[id].json"));

        assert!(!guess_if_route_is_endpoint("/"));
        assert!(!guess_if_route_is_endpoint("/articles"));
        assert!(!guess_if_route_is_endpoint("/articles/[slug]"));
        assert!(!guess_if_route_is_endpoint("/blog/posts/[year]/[month]"));
    }
}
