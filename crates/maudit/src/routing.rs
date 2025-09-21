use std::path::Path;

use crate::page::RouteType;

#[derive(Debug, PartialEq)]
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

    params
}

pub fn get_route_type_from_route_params(params_def: &[ParameterDef]) -> RouteType {
    if params_def.is_empty() {
        RouteType::Static
    } else {
        RouteType::Dynamic
    }
}

pub fn guess_if_route_is_endpoint(raw_route: &str) -> bool {
    let real_path = Path::new(&raw_route);

    real_path.extension().is_some()
}

#[cfg(test)]
mod tests {
    use crate::{
        page::RouteType,
        routing::{ParameterDef, extract_params_from_raw_route, get_route_type_from_route_params},
    };

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
                key: "article".to_string(),
                index: 10,
                length: 9,
            },
            ParameterDef {
                key: "id".to_string(),
                index: 20,
                length: 4,
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
    fn test_route_type_static() {
        let input = "/articles";
        let params_def = extract_params_from_raw_route(input);
        assert_eq!(
            get_route_type_from_route_params(&params_def),
            RouteType::Static
        );
    }

    #[test]
    fn test_route_type_dynamic() {
        let input = "/articles/[article]";
        let params_def = extract_params_from_raw_route(input);
        assert_eq!(
            get_route_type_from_route_params(&params_def),
            RouteType::Dynamic
        );
    }

    #[test]
    fn test_route_type_dynamic_multiple() {
        let input = "/articles/[article]/[id]";
        let params_def = extract_params_from_raw_route(input);
        assert_eq!(
            get_route_type_from_route_params(&params_def),
            RouteType::Dynamic
        );
    }

    #[test]
    fn test_route_type_dynamic_escaped() {
        let input = "/articles/\\[article\\]";
        let params_def = extract_params_from_raw_route(input);
        assert_eq!(
            get_route_type_from_route_params(&params_def),
            RouteType::Static
        );
    }

    #[test]
    fn test_route_type_dynamic_mixed_escaped_brackets() {
        let input = "/articles/\\[article\\]/[id]";
        let params_def = extract_params_from_raw_route(input);
        assert_eq!(
            get_route_type_from_route_params(&params_def),
            RouteType::Dynamic
        );
    }
}
