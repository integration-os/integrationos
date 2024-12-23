use serde_json::Value;

pub fn match_route<'a>(
    full_path: &'a str,
    routes: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
    let path = full_path.split('?').next().unwrap_or("");

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    for route in routes {
        let route_segments: Vec<&str> = route.split('/').filter(|s| !s.is_empty()).collect();

        if segments.len() != route_segments.len() {
            continue;
        }

        if route_segments
            .iter()
            .zip(&segments)
            .all(|(route_seg, path_seg)| {
                route_seg == path_seg
                    || route_seg.starts_with(':')
                    || (route_seg.starts_with("{{") && route_seg.ends_with("}}"))
            })
        {
            return Some(route);
        }
    }

    None
}

pub fn template_route(model_definition_path: String, full_request_path: String) -> String {
    let model_definition_segments: Vec<&str> = model_definition_path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let full_request_segments: Vec<&str> = full_request_path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let mut template = String::new();

    for (i, segment) in model_definition_segments.iter().enumerate() {
        if segment.starts_with(':') || (segment.starts_with("{{") && segment.ends_with("}}")) {
            template.push_str(full_request_segments[i]);
        } else {
            template.push_str(segment);
        }

        if i != model_definition_segments.len() - 1 {
            template.push('/');
        }
    }

    template
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_route() {
        let routes = [
            "/customers",
            "/customers/:id",
            "/customers/{{id}}/orders",
            "/customers/:id/orders/:order_id",
        ]
        .into_iter();

        assert_eq!(
            match_route("/customers", routes.clone()),
            Some("/customers")
        );
        assert_eq!(
            match_route("/customers/123", routes.clone()),
            Some("/customers/:id")
        );
        assert_eq!(
            match_route("/customers/123/orders", routes.clone()),
            Some("/customers/{{id}}/orders")
        );
        assert_eq!(
            match_route("/customers/123/orders/456", routes.clone()),
            Some("/customers/:id/orders/:order_id")
        );
        assert_eq!(match_route("/customers/123/456", routes.clone()), None);
        assert_eq!(match_route("/customers/123/orders/456/789", routes), None);
    }

    #[test]
    fn test_template_route() {
        assert_eq!(
            template_route(
                "/customers/:id/orders/:order_id".to_string(),
                "/customers/123/orders/456".to_string()
            ),
            "customers/123/orders/456".to_string()
        );

        assert_eq!(
            template_route(
                "/customers/{{id}}/orders/{{order_id}}".to_string(),
                "/customers/123/orders/456".to_string()
            ),
            "customers/123/orders/456".to_string()
        );
    }
}
