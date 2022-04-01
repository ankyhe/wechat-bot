use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use chrono::Utc;
use log::info;
use lazy_static::lazy_static;
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Client {
    uuid: String,
    lang: String,
    scan: String,
    ticket: String,
    tip: i32,
}

const WECHAT_WEB_APP_ID: &str = "wx782c26e4c19acffb";
const QR_CODE_URL: &str = "https://wx.qq.com/jslogin";
const QR_CODE_SCAN_RESULT_URL: &str = "https://wx.qq.com/cgi-bin/mmwebwx-bin/login";

impl Client {
    pub fn new() -> Self {
        Client {
            uuid: String::default(),
            lang: "zh_CN".to_string(),
            scan: String::default(),
            ticket: String::default(),
            tip: 1,
        }
    }

    fn set_from_map(&mut self, map: &HashMap<String, String>) -> &Self {
        if map.contains_key("uuid") {
            self.uuid = map.get("uuid").unwrap().to_string();
        }
        if map.contains_key("lang") {
            self.lang = map.get("lang").unwrap().to_string();
        }
        if map.contains_key("scan") {
            self.scan = map.get("scan").unwrap().to_string();
        }
        if map.contains_key("ticket") {
            self.ticket = map.get("ticket").unwrap().to_string();
        }
        self
    }

    pub async fn retrieve_qr_code(&mut self) -> Result<(), Box<dyn Error>> {
        let params = QrCodeRetrieveParams {
            appid: WECHAT_WEB_APP_ID.into(),
            fun: "new".into(),
            lang: "zh_CN".into(),
            now: Utc::now().timestamp_millis()
        };
        let http_client = reqwest::Client::new();
        let resp_body = http_client.post(QR_CODE_URL)
            .form(&params)
            .send()
            .await?
            .text()
            .await?;
        info!("<== [{}] body: [{:?}] client: [{:?}]", QR_CODE_URL, resp_body, self);
        Client::process_qr_code_response(&resp_body[..]).and_then(|scan_code_uuid| {
            self.uuid = scan_code_uuid;
            info!("please login with: https://wx.qq.com/qrcode/{}", self.uuid);
            Ok(())
        })
    }

    fn process_qr_code_response(resp: &str) -> Result<String, Box<dyn Error>> {
        // The response example:
        // "window.QRLogin.code = 200; window.QRLogin.uuid = \"AZJIzIcS5g==\";"
        let sections = resp.split(";");
        let mut ret: String = String::default();
        let mut ret_code = false;
        for section in sections {
            let used_section = section.trim();
            let parts = used_section.split(" = ").collect::<Vec<&str>>();
            if parts.len() < 0 {
                break;
            }
            if parts[0] == "window.QRLogin.code" {
                if parts[1] == "200" {
                    ret_code = true;
                }
            } else if parts[0] == "window.QRLogin.uuid" {
                let tmp = parts[1];
                ret = tmp[1..tmp.len() - 1].to_string();
            }
        }
        if ret_code {
            Ok(ret)
        } else {
            Err("The response code is not 200".into())
        }
    }

    pub async fn get_qr_code_scan_result(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.get_qr_code_scan_result_impl().await {
                Ok(_) => {
                    if self.tip < 0 {
                        return Ok(());
                    }
                    continue;
                },
                Err(err) => {return Err(err);}
            }
        }
    }

    async fn get_qr_code_scan_result_impl(&mut self) -> Result<(), Box<dyn Error>> {
        // thread::sleep(Duration::from_secs(20));
        let params = QrCodeScanResultParams {
            tip: self.tip,
            uuid: self.uuid.to_string(),
            now: Utc::now().timestamp_millis(),
            loginicon: true /* we don't need avatar */
        };
        info!("==> [{}] query: [{:?}] client: [{:?}]", QR_CODE_SCAN_RESULT_URL, params, self);
        let http_client = reqwest::Client::new();
        let query = serde_urlencoded::to_string(params).unwrap();
        let resp_body = http_client.get(format!("{}?{}", QR_CODE_SCAN_RESULT_URL, query))
            .send()
            .await?
            .text()
            .await?;
        info!("<== [{}] body: [{:?}] client: [{:?}]", QR_CODE_SCAN_RESULT_URL, resp_body, self);
        self.tip = -1;
        Ok(())
        //self.process_qr_code_scan_result_response(&resp_body)
    }

    fn process_qr_code_scan_result_response(&mut self, resp: &HashMap<String, String>) -> Result<(), Box<dyn Error>> {
        // The response example:
        // {
        //    'window.code': '201'
        // }
        // {
        //     'window.code': '200',
        //     'window.redirect_uri': 'https://wx.qq.com/cgi-bin/mmwebwx-bin/webwxnewloginpage?ticket=ARD37_ikx-Kakd2i0W-f-E7q@qrticket_0&uuid=4f6yOkV4AA==&lang=zh_CN&scan=1548300672' }
        // }
        lazy_static! {
            static ref STRING_DEF: String = String::default();
        }
        let window_code = resp.get("window.code").unwrap_or(&STRING_DEF);
        if self.tip == 1 && window_code == "201" {
            self.tip = 0;
            return Ok(());
        }
        if self.tip == 0 && window_code == "200" {
            self.tip = -1;
            let redirect_url = resp.get("window.redirect_url").unwrap_or(&STRING_DEF);
            if redirect_url.is_empty() {
                return Err("Failed to retrieve redirect_url".into());
            }
            let url = Url::from_str(redirect_url)?;
            let redirect_params = serde_urlencoded::from_str::<HashMap<String, String>>(url.query().unwrap())?;
            self.set_from_map(&redirect_params);
            return Ok(());
        }
        Ok(())
    }

    fn process_redirect_url(redirect_url: &String) -> Result<HashMap<String, String>, Box<dyn Error>> {
        let url = Url::from_str(redirect_url)?;
        match serde_urlencoded::from_str::<HashMap<String, String>>(url.query().unwrap_or_default()) {
            Ok(result) => return Ok(result),
            Err(err) => return Err(format!("Failed to parse from redirect_url: {} due to {:?}.", redirect_url, err.to_string()).into())
        }
    }
}

/* impl */
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct QrCodeRetrieveParams {
    appid: String,
    fun: String,
    lang: String,
    #[serde(rename(serialize = "_"))]
    now: i64
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct QrCodeScanResultParams {
    tip: i32,
    uuid: String,
    #[serde(rename(serialize = "_"))]
    now: i64,
    loginicon: bool
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use crate::Client;

    fn init() {
        let _ = env_logger::builder().is_test(false).try_init();

    }

    #[tokio::test]
    async fn test_client() {
        init();
        let mut client = Client::new();
        client.retrieve_qr_code().await.unwrap();
        println!("qr_uuid is {}", client.uuid);
        assert!(!client.uuid.is_empty());
        client.get_qr_code_scan_result().await.unwrap();
    }

    #[test]
    fn test_process_redirect_url() {
        let redirect_url = String::from("https://wx.qq.com/cgi-bin/mmwebwx-bin/webwxnewloginpage?ticket=ARD37_ikx-Kakd2i0W-f-E7q@qrticket_0&uuid=4f6yOkV4AA==&lang=zh_CN&scan=1548300672");
        let result = Client::process_redirect_url(&redirect_url).unwrap();
        println!("result is {:?}", result);
    }
}
