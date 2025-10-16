pub fn inject_hmr_script(html: &str) -> String {
    const HMR_SCRIPT: &str = include_str!("../hmr/hmr_client.js");

    let script_tag = format!("<script type=\"module\">{}</script>", HMR_SCRIPT);

    if let Some(pos) = html.find("</head>") {
        let mut result = String::with_capacity(html.len() + script_tag.len());
        result.push_str(&html[..pos]);
        result.push_str(&script_tag);
        result.push_str(&html[pos..]);
        result
    } else if let Some(pos) = html.find("</body>") {
        let mut result = String::with_capacity(html.len() + script_tag.len());
        result.push_str(&html[..pos]);
        result.push_str(&script_tag);
        result.push_str(&html[pos..]);
        result
    } else {
        format!("{}{}", html, script_tag)
    }
}
