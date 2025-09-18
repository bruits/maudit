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
    let mut start = false;
    let mut escape = false;
    let mut current_value = String::new();

    for (i, c) in raw_route.char_indices() {
        if escape {
            escape = false;
            if start {
                current_value.push(c);
            }
            continue;
        }

        match c {
            '\\' => {
                escape = true;
            }
            '[' => {
                if !escape {
                    start = true;
                    current_value.clear();
                }
            }
            ']' => {
                if start {
                    params.push(ParameterDef {
                        key: current_value.clone(),
                        index: i - (current_value.len() + 1), // -1 for the starting [
                        length: current_value.len() + 2,      // +2 for the [ and ]
                    });
                    start = false;
                }
            }
            _ => {
                if start {
                    current_value.push(c);
                }
            }
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
        route::{ParameterDef, extract_params_from_raw_route, get_route_type_from_route_params},
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
