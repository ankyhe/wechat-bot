use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use lazy_static::lazy_static;
use reqwest::Url;
use serde::ser;

/*
 * append query params to url.
 */
pub fn append_query_to_url<T: ser::Serialize>(url: &str, params: T) -> String {
    let query = serde_urlencoded::to_string(params).unwrap_or(String::default());
    if query.is_empty() {
        return url.to_string();
    }
    format!("{}?{}", url, query)
}

pub fn process_redirect_url(redirect_url: &String) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let url = Url::from_str(redirect_url)?;
    match serde_urlencoded::from_str::<HashMap<String, String>>(url.query().unwrap_or_default()) {
        Ok(result) => return Ok(result),
        Err(err) => return Err(format!("Failed to parse from redirect_url: {} due to {:?}.", redirect_url, err.to_string()).into())
    }
}

/*
 * text =>  "window.QRLogin.code = 200; window.QRLogin.uuid = \"AZJIzIcS5g==\";"
 * ret => {"window.QRLogin.code": "200", "window.QRLogin.uuid": "AZJIzIcS5g=="}
 */
pub fn text_to_map(text: &str) -> HashMap<String, String> {
    lazy_static! {
        static ref SPACE_REGEX: Regex = Regex::new(r"\s*=\s*").unwrap();
    }
    let sections = text.split(";");
    let mut ret: HashMap<String, String> = HashMap::new();
    for section in sections {
        let mut used_section = section.trim();
        if used_section.starts_with("\\n") {
            used_section = &used_section[1..];
        }
        let parts = SPACE_REGEX.splitn(used_section, 2).collect::<Vec<&str>>();
        if parts.len() < 2 {
            break;
        }
        let mut value = parts[1];
        if parts[1].starts_with("\"") && parts[1].ends_with("\"") {
            value = &value[1..value.len() - 1];
        }
        ret.insert(parts[0].to_string(), value.to_string());
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_redirect_url() {
        let redirect_url = String::from("https://wx.qq.com/cgi-bin/mmwebwx-bin/webwxnewloginpage?ticket=ARD37_ikx-Kakd2i0W-f-E7q@qrticket_0&uuid=4f6yOkV4AA==&lang=zh_CN&scan=1548300672");
        let result = process_redirect_url(&redirect_url).unwrap();
        println!("result is {:?}", result);
    }

    #[test]
    fn test_text_to_map() {
        let result = text_to_map("window.QRLogin.code = 200; window.QRLogin.uuid = \"oeZfYakK6g==\";");
        let mut expected: HashMap<String, String> = HashMap::new();
        expected.insert("window.QRLogin.code".to_string(), "200".to_string());
        expected.insert("window.QRLogin.uuid".to_string(), "oeZfYakK6g==".to_string());
        assert_eq!(expected, result);

        let result = text_to_map("window.code=201;");
        let mut expected: HashMap<String, String> = HashMap::new();
        expected.insert("window.code".to_string(), "201".to_string());
        assert_eq!(expected, result);

        let result = text_to_map("window.code=200;\nwindow.redirect_uri=\"https://wx.qq.com/cgi-bin/mmwebwx-bin/webwxnewloginpage?ticket=AVTK4m8A8ThyfrYZKuoHiY6i@qrticket_0&uuid=YZeXOrjTMw==&lang=zh_CN&scan=1648884679\";");
        expected.insert("window.code".to_string(), "200".to_string());
        expected.insert("window.redirect_uri".to_string(), "https://wx.qq.com/cgi-bin/mmwebwx-bin/webwxnewloginpage?ticket=AVTK4m8A8ThyfrYZKuoHiY6i@qrticket_0&uuid=YZeXOrjTMw==&lang=zh_CN&scan=1648884679".to_string());
        assert_eq!(expected, result);
    }
}
