use super::*;

pub struct RouteHandler;
impl RouteHandler{
    // Find fn pointer to execute the route given the Http-method and path from the incoming
    // request.
    // We pass the TcpStream in case a WebSocket-endpoint is called.
    pub async fn goto(req: &mut http::Http, psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {

        if req.method == "OPTIONS" {
            return Ok(Routes::handle_preflight().await);
        }

        if let Some(r) = ROUTES.iter().find(|&&(m, p, _)| {
            let path_matches = match check_path(p, &*req.path) {
                Ok(matches) => matches,
                Err(_) => false,
            };

            m == req.method && path_matches
        }) {
            let (_, p, ro) = *r;
            req.params = extract_params(p, &*req.path);
            return ro(&Routes, req.clone(), psql, api).await;
        } else {
            return Ok(Routes::not_found().await);
        }

        // A route can contain custom parameters e.g /user/{name}. By formatting
        // the defined routes into a regex string we can check if the defined path
        // matches the given path incoming from the request.
        //
        // :param p: Path we predefined in our routes.
        // :param path: Path extracted from the incoming request.
        fn check_path(p: &str, path: &str) -> Result<bool, http::Error> {
            
            let re_custom_param = Regex::new(r"\{(\w+)\}").unwrap();
            //let formatted_path = re_custom_param.replace_all(p, "(\\w+)");
            let formatted_path = re_custom_param.replace_all(p, "([\\w%:.+-]+)");
            let regex_str_path = format!("^{}$", formatted_path);

            let re = Regex::new(&regex_str_path)?;
            Ok(re.is_match(path))
        }

        // Extract the parameters from the path given in the request and make it
        // workable in a HashMap.
        // E.g:
        // p = /user/{firstname}/{lastname}, path = /user/foo/bar, becomes:
        // {"firstname": "foo", "lastname": "bar"}
        //
        // :param p: Path we predefined in our routes.
        // :param path: Path extracted from the incoming request.
        fn extract_params(p: &str, path: &str) -> std::collections::HashMap<String, String> {
            let mut params : std::collections::HashMap<String, String> = std::collections::HashMap::new();

            // Split both paths in pieces.
            let p_pieces = p.split("/");
            let path_pieces = path.split("/").collect::<std::vec::Vec<&str>>();

            // Loop over the pieces of our predefined route path. If a piece
            // contains {...} AKA is a custom parameter:
            // Add the string between the brackets as key and add the value of the
            // path from the request as value to the hashmap.
            let mut i = 0;
            for p_piece in p_pieces {
                let re = Regex::new(r"\{(\w+)\}").unwrap();
                if re.is_match(p_piece) {
                    let param_name = p_piece.replace("{", "").replace("}", "");
                    params.insert(param_name, path_pieces[i].into());
                }

                i = i + 1;
            }

            params
        }
    }
}

