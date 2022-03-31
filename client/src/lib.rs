use std::error::Error;
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub struct Client {
}

const WECHAT_WEB_APP_ID: &str = "wx782c26e4c19acffb";
const QR_CODE_URL: &str = "https://wx.qq.com/jslogin";

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    pub async fn retrieve_qrcode(&self) -> Result<String, Box<dyn Error>> {
        let params = Params {
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
        // The response example:
        // "window.QRLogin.code = 200; window.QRLogin.uuid = \"AZJIzIcS5g==\";"
        let sections = resp_body.split(";");
        let mut ret: String = String::default();
        let mut ret_code = false;
        for section in sections {
            let used_section = section.trim();
            let parts = used_section.split(" = ").collect::<Vec<&str>>();
            if parts.len() < 2 {
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
}

/* impl */
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Params {
    appid: String,
    fun: String,
    lang: String,
    #[serde(rename(serialize = "_"))]
    now: i64
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[tokio::test]
    async fn test_client() {
        let client = Client::new();
        let qr_uuid = client.retrieve_qrcode().await.unwrap();
        println!("qr_uuid is {}", qr_uuid);
        assert!(!qr_uuid.is_empty())
    }
}
