use crate::config_helper::CookieJar;
use crate::x_bogus_js;
use quick_js::Context;
use rand::{thread_rng, Rng};
use reqwest::header::{CONTENT_TYPE, COOKIE, USER_AGENT};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36 Edg/122.0.0.0";

trait Monitor {
    fn start_once(
        &self,
        url: &str,
        cookies: Option<&str>,
        show_offset: Option<&str>,
        is_new_offset: Option<&str>,
    ) -> (Vec<NewestVideo>, String);
}

#[derive(Debug)]
pub struct NewestVideo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub date: String,
    pub is_new: bool,
}

pub fn get_newest_video(
    url: &str,
    cookies: Option<CookieJar>,
    show_offset: Option<&str>,
    is_new_offset: Option<&str>,
) -> Result<(Vec<NewestVideo>, String), std::io::Error> {
    let mut _cookies = None;

    let _url = Url::parse(url).unwrap();
    let monitor_instance = match _url.host_str().unwrap() {
        "space.bilibili.com" => {
            _cookies = cookies
                .as_ref()
                .and_then(|c| c.bilibili.as_ref().map(|c| c.as_str()));
            get_bilibili_monitor_instance()
        }
        "www.kuaishou.com" => {
            _cookies = cookies
                .as_ref()
                .and_then(|c| c.kuaishou.as_ref().map(|c| c.as_str()));
            get_kuaishou_monitor_instance()
        }
        "www.ixigua.com" => {
            _cookies = cookies
                .as_ref()
                .and_then(|c| c.ixigua.as_ref().map(|c| c.as_str()));
            get_ixigua_monitor_instance()
        }
        "www.douyin.com" => {
            _cookies = cookies
                .as_ref()
                .and_then(|c| c.douyin.as_ref().map(|c| c.as_str()));
            get_douyin_monitor_instance()
        }
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unsupported host: {}", _url.host_str().unwrap()),
            ))
        }
    };

    Ok(monitor_instance.start_once(url, _cookies, show_offset, is_new_offset))
}

/// ================================================================================================
/// Bilibili
/// ================================================================================================

const BILIBILI_MONITOR_API: &str = "https://api.bilibili.com/x/space/wbi/arc/search";
const BILIBILI_REFERER: &str = "https://space.bilibili.com/";

static mut BILIBILI_WBI_KEYS: Option<(String, String)> = None;

static mut BILIBILI_MONITOR_INSTANCE: Option<Box<dyn Monitor>> = None;

fn get_bilibili_monitor_instance() -> &'static Box<dyn Monitor> {
    unsafe {
        if BILIBILI_MONITOR_INSTANCE.is_none() {
            BILIBILI_MONITOR_INSTANCE = Some(Box::new(BilibiliMonitor));
        }
        BILIBILI_MONITOR_INSTANCE.as_ref().unwrap()
    }
}

/// Bilibili monitor
struct BilibiliMonitor;
impl Monitor for BilibiliMonitor {
    fn start_once(
        &self,
        url: &str,
        cookies: Option<&str>,
        show_offset: Option<&str>,
        is_new_offset: Option<&str>,
    ) -> (Vec<NewestVideo>, String) {
        // e.g. https://space.bilibili.com/1344420936?spm_id_from=333.1007.tianma.1-1-1.click
        let _url = Url::parse(url).unwrap();
        // e.g. /1344420936
        let path = _url.path();
        // e.g. 1344420936
        let mid = &path[1..path.len()];

        let (dm_img_str, dm_cover_img_str) = self.gen_random_dm();

        // url query parameters
        let mut params = vec![
            ("mid", mid.to_string()),
            ("pn", "1".to_string()),
            ("ps", "10".to_string()),
            ("index", "1".to_string()),
            ("order", "pubdate".to_string()),
            ("order_avoided", "true".to_string()),
            ("platform", "web".to_string()),
            ("web_location", "1550101".to_string()),
            ("dm_img_list", "[]".to_string()),
            ("dm_img_str", dm_img_str),
            ("dm_cover_img_str", dm_cover_img_str),
            (
                "dm_img_inter",
                r#"{"ds":[],"wh":[0,0,0],"of":[0,0,0]}"#.to_string(),
            ),
        ];
        let keys = self.get_wbi_keys(cookies.unwrap_or("")).unwrap();
        let query = self.encode_wbi(&mut params, keys);

        let api = format!("{}?{}", BILIBILI_MONITOR_API, query);
        let referer = format!("{}{}/video", BILIBILI_REFERER, mid);

        // get the newest video
        let response = reqwest::blocking::Client::new()
            .get(&api)
            // referer
            .header("referer", referer)
            // user agent
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            // cookies
            .header(COOKIE, cookies.unwrap_or(""))
            .send()
            .unwrap();

        let json: serde_json::Value = response.json().unwrap();

        let mut videos = Vec::new();
        let mut next_offset: u64 = 0;

        // data -> list -> vlist
        if let Some(vlist) = json["data"]["list"]["vlist"].as_array() {
            for video in vlist {
                let id = video["bvid"].as_str().unwrap();
                let title = video["title"].as_str().unwrap();
                let url = format!("https://www.bilibili.com/video/{}", id);
                let date = video["created"].as_u64().unwrap() * 1000;

                // offset
                if let Some(offset) = show_offset {
                    if date <= offset.parse::<u64>().unwrap() {
                        continue;
                    }
                }

                let is_new = if let Some(offset) = is_new_offset {
                    date > offset.parse::<u64>().unwrap()
                } else {
                    true
                };

                videos.push(NewestVideo {
                    id: id.to_string(),
                    title: title.to_string(),
                    url,
                    date: date.to_string(),
                    is_new,
                });

                if next_offset == 0 {
                    next_offset = date;
                } else if date > next_offset {
                    next_offset = date;
                }
            }
        } else {
            println!("{:?}", json);
        }

        (videos, next_offset.to_string())
    }
}

#[derive(Deserialize)]
struct WbiImg {
    img_url: String,
    sub_url: String,
}

#[derive(Deserialize)]
struct Data {
    wbi_img: WbiImg,
}

#[derive(Deserialize)]
struct ResWbi {
    data: Data,
}

impl BilibiliMonitor {
    fn gen_mixin_key(&self, raw_wbi_key: impl AsRef<[u8]>) -> String {
        const MIXIN_KEY_ENC_TAB: [u8; 64] = [
            46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42,
            19, 29, 28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60,
            51, 30, 4, 22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
        ];
        let raw_wbi_key = raw_wbi_key.as_ref();
        let mut mixin_key = {
            let binding = MIXIN_KEY_ENC_TAB
                .iter()
                // 此步操作即遍历 MIXIN_KEY_ENC_TAB，取出 raw_wbi_key 中对应位置的字符
                .map(|n| raw_wbi_key[*n as usize])
                // 并收集进数组内
                .collect::<Vec<u8>>();
            unsafe { String::from_utf8_unchecked(binding) }
        };
        let _ = mixin_key.split_off(32); // 截取前 32 位字符
        mixin_key
    }

    fn get_url_encoded(&self, s: &str) -> String {
        s.chars()
            .filter_map(|c| match c.is_ascii_alphanumeric() || "-_.~".contains(c) {
                true => Some(c.to_string()),
                false => {
                    // 过滤 value 中的 "!'()*" 字符
                    if "!'()*".contains(c) {
                        return None;
                    }
                    let encoded = c
                        .encode_utf8(&mut [0; 4])
                        .bytes()
                        .fold("".to_string(), |acc, b| acc + &format!("%{:02X}", b));
                    Some(encoded)
                }
            })
            .collect::<String>()
    }

    // 为请求参数进行 wbi 签名
    fn encode_wbi(
        &self,
        params: &mut Vec<(&str, String)>,
        (img_key, sub_key): (String, String),
    ) -> String {
        let mixin_key = self.gen_mixin_key((img_key + &sub_key).as_bytes());
        let cur_time = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(t) => t.as_secs(),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };
        // 添加当前时间戳
        params.push(("wts", cur_time.to_string()));
        // 重新排序
        params.sort_by(|a, b| a.0.cmp(b.0));
        let query = params.iter().fold(String::from(""), |acc, (k, v)| {
            acc + format!("{}={}&", self.get_url_encoded(k), self.get_url_encoded(v)).as_str()
        });

        // query截取最后一个字符
        let query = query.trim_end_matches('&').to_string();

        let web_sign = format!("{:?}", md5::compute(query.clone() + &mixin_key));

        query + &format!("&w_rid={}", web_sign)
    }

    fn get_wbi_keys(&self, cookies: &str) -> Result<(String, String), reqwest::Error> {
        // if BILIBILI_WBI_KEYS is not None, return it
        unsafe {
            if BILIBILI_WBI_KEYS.is_some() {
                return Ok(BILIBILI_WBI_KEYS.clone().unwrap());
            }
        }

        // get wbi keys
        let client = reqwest::blocking::Client::new();
        let ResWbi {
            data: Data { wbi_img },
        } = client
            .get("https://api.bilibili.com/x/web-interface/nav")
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            // SESSDATA=xxxxx
            .header("Cookie", cookies)
            .send()
            .unwrap()
            .json::<ResWbi>()
            .unwrap();

        let img_key = wbi_img
            .img_url
            .split('/')
            .last()
            .unwrap()
            .split('.')
            .next()
            .unwrap();
        let sub_key = wbi_img
            .sub_url
            .split('/')
            .last()
            .unwrap()
            .split('.')
            .next()
            .unwrap();

        // save to static variable
        unsafe {
            BILIBILI_WBI_KEYS = Some((img_key.to_string(), sub_key.to_string()));
        }

        Ok((img_key.to_string(), sub_key.to_string()))
    }

    fn gen_random_dm(&self) -> (String, String) {
        fn gen_random_str() -> String {
            let source_string = "ABCDEFGHIJK";
            let mut rng = thread_rng();
            let random_index_1 = rng.gen_range(0..source_string.len());
            let random_index_2 = rng.gen_range(0..source_string.len());
            let random_char_1 = source_string.chars().nth(random_index_1).unwrap();
            let random_char_2 = source_string.chars().nth(random_index_2).unwrap();
            let random_string = format!("{}{}", random_char_1, random_char_2);
            random_string
        }

        (gen_random_str(), gen_random_str())
    }
}

/// ================================================================================================
/// Kuaishou
/// ================================================================================================

const KUAISHOU_MONITOR_API: &str = "https://www.kuaishou.com/graphql";
const KUAISHOU_REFERER: &str = "https://www.kuaishou.com/profile/";

static mut KUAISHOU_MONITOR_INSTANCE: Option<Box<dyn Monitor>> = None;

fn get_kuaishou_monitor_instance() -> &'static Box<dyn Monitor> {
    unsafe {
        if KUAISHOU_MONITOR_INSTANCE.is_none() {
            KUAISHOU_MONITOR_INSTANCE = Some(Box::new(KuaishouMonitor));
        }
        KUAISHOU_MONITOR_INSTANCE.as_ref().unwrap()
    }
}

/// Kuaishou monitor
struct KuaishouMonitor;
impl Monitor for KuaishouMonitor {
    fn start_once(
        &self,
        url: &str,
        cookies: Option<&str>,
        show_offset: Option<&str>,
        is_new_offset: Option<&str>,
    ) -> (Vec<NewestVideo>, String) {
        // e.g. https://www.kuaishou.com/profile/3xxcvi49q2r52gu
        let _url = Url::parse(url).unwrap();
        // e.g. /profile/3xxcvi49q2r52gu
        let path = _url.path();
        // e.g. 3xxcvi49q2r52gu
        let id = &path[9..path.len()];

        let body = r#"
{
    "operationName": "visionProfilePhotoList",
    "variables": {
        "userId": "{}",
        "pcursor": "",
        "page": "profile"
    },
    "query": "fragment photoContent on PhotoEntity {\n  __typename\n  id\n  duration\n  caption\n  originCaption\n  likeCount\n  viewCount\n  commentCount\n  realLikeCount\n  coverUrl\n  photoUrl\n  photoH265Url\n  manifest\n  manifestH265\n  videoResource\n  coverUrls {\n    url\n    __typename\n  }\n  timestamp\n  expTag\n  animatedCoverUrl\n  distance\n  videoRatio\n  liked\n  stereoType\n  profileUserTopPhoto\n  musicBlocked\n  riskTagContent\n  riskTagUrl\n}\n\nfragment recoPhotoFragment on recoPhotoEntity {\n  __typename\n  id\n  duration\n  caption\n  originCaption\n  likeCount\n  viewCount\n  commentCount\n  realLikeCount\n  coverUrl\n  photoUrl\n  photoH265Url\n  manifest\n  manifestH265\n  videoResource\n  coverUrls {\n    url\n    __typename\n  }\n  timestamp\n  expTag\n  animatedCoverUrl\n  distance\n  videoRatio\n  liked\n  stereoType\n  profileUserTopPhoto\n  musicBlocked\n  riskTagContent\n  riskTagUrl\n}\n\nfragment feedContent on Feed {\n  type\n  author {\n    id\n    name\n    headerUrl\n    following\n    headerUrls {\n      url\n      __typename\n    }\n    __typename\n  }\n  photo {\n    ...photoContent\n    ...recoPhotoFragment\n    __typename\n  }\n  canAddComment\n  llsid\n  status\n  currentPcursor\n  tags {\n    type\n    name\n    __typename\n  }\n  __typename\n}\n\nquery visionProfilePhotoList($pcursor: String, $userId: String, $page: String, $webPageArea: String) {\n  visionProfilePhotoList(pcursor: $pcursor, userId: $userId, page: $page, webPageArea: $webPageArea) {\n    result\n    llsid\n    webPageArea\n    feeds {\n      ...feedContent\n      __typename\n    }\n    hostName\n    pcursor\n    __typename\n  }\n}\n"
}
        "#;

        let response = reqwest::blocking::Client::new()
            .post(KUAISHOU_MONITOR_API)
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .header("referer", format!("{}{}", KUAISHOU_REFERER, id))
            .header(COOKIE, cookies.unwrap_or(""))
            .header(CONTENT_TYPE, "application/json")
            .body(body.replace("{}", id))
            .send()
            .unwrap();

        let json: serde_json::Value = response
            .json()
            .and_then(|j| Ok(j))
            .unwrap_or(serde_json::Value::Null);

        let mut videos = Vec::new();
        let mut next_offset: u64 = 0;

        // data -> visionProfilePhotoList -> feeds
        if let Some(vlist) = json["data"]["visionProfilePhotoList"]["feeds"].as_array() {
            for video in vlist {
                let id = video["photo"]["id"].as_str().unwrap();
                let title = video["photo"]["caption"].as_str().unwrap();
                let url = format!("https://www.kuaishou.com/short-video/{}", id);
                let date = video["photo"]["timestamp"].as_u64().unwrap();

                // offset
                if let Some(offset) = show_offset {
                    if date <= offset.parse::<u64>().unwrap() {
                        continue;
                    }
                }

                let is_new = if let Some(offset) = is_new_offset {
                    date > offset.parse::<u64>().unwrap()
                } else {
                    true
                };

                videos.push(NewestVideo {
                    id: id.to_string(),
                    title: title.to_string(),
                    url,
                    date: date.to_string(),
                    is_new,
                });

                if next_offset == 0 {
                    next_offset = date;
                } else if date > next_offset {
                    next_offset = date;
                }
            }
        } else {
            println!("{:?}", json);
        }

        (videos, next_offset.to_string())
    }
}

/// ================================================================================================
/// IXigua
/// ================================================================================================

const IXIGUA_MONITOR_API: &str = "https://www.ixigua.com/api/videov2/author/new_video_list";

static mut IXIGUA_MONITOR_INSTANCE: Option<Box<dyn Monitor>> = None;

fn get_ixigua_monitor_instance() -> &'static Box<dyn Monitor> {
    unsafe {
        if IXIGUA_MONITOR_INSTANCE.is_none() {
            IXIGUA_MONITOR_INSTANCE = Some(Box::new(IXiguaMonitor));
        }
        IXIGUA_MONITOR_INSTANCE.as_ref().unwrap()
    }
}

struct IXiguaMonitor;
impl Monitor for IXiguaMonitor {
    fn start_once(
        &self,
        url: &str,
        cookies: Option<&str>,
        show_offset: Option<&str>,
        is_new_offset: Option<&str>,
    ) -> (Vec<NewestVideo>, String) {
        // https://www.ixigua.com/home/2497727299858013/
        let _url = Url::parse(url).unwrap();
        // /home/2497727299858013/
        let path = _url.path();
        // 2497727299858013
        let mut id = &path[6..path.len()];
        // if last char is '/', remove it
        if id.ends_with('/') {
            id = &id[0..id.len() - 1];
        }

        let response = reqwest::blocking::Client::new()
            .get(IXIGUA_MONITOR_API)
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .header("referer", url)
            .header(COOKIE, cookies.unwrap_or(""))
            .query(&[
                ("to_user_id", id),
                ("offset", "0"),
                ("limit", "10"),
                ("order", "new"),
            ])
            .send()
            .unwrap();

        let json: serde_json::Value = response
            .json()
            .and_then(|j| Ok(j))
            .unwrap_or(serde_json::Value::Null);

        let mut videos = Vec::new();
        let mut next_offset: u64 = 0;

        if let Some(vlist) = json["data"]["videoList"].as_array() {
            for video in vlist {
                let id = video["item_id"].as_str().unwrap();
                let title = video["title"].as_str().unwrap();
                let url = format!("https://www.ixigua.com/{}", id);
                let date = video["publish_time"].as_u64().unwrap() * 1000;

                // offset
                if let Some(offset) = show_offset {
                    if date <= offset.parse::<u64>().unwrap() {
                        continue;
                    }
                }

                let is_new = if let Some(offset) = is_new_offset {
                    date > offset.parse::<u64>().unwrap()
                } else {
                    true
                };

                videos.push(NewestVideo {
                    id: id.to_string(),
                    title: title.to_string(),
                    url,
                    date: date.to_string(),
                    is_new,
                });

                if next_offset == 0 {
                    next_offset = date;
                } else if date > next_offset {
                    next_offset = date;
                }
            }
        } else {
            println!("{:?}", json);
        }

        (videos, next_offset.to_string())
    }
}

///=================================================================================================
/// Douyin
///=================================================================================================

const DOUYIN_MONITOR_API: &str = "https://www.douyin.com/aweme/v1/web/aweme/post/";

static mut DOUYIN_MONITOR_INSTANCE: Option<Box<dyn Monitor>> = None;

fn get_douyin_monitor_instance() -> &'static Box<dyn Monitor> {
    unsafe {
        if DOUYIN_MONITOR_INSTANCE.is_none() {
            DOUYIN_MONITOR_INSTANCE = Some(Box::new(DouyinMonitor));
        }
        DOUYIN_MONITOR_INSTANCE.as_ref().unwrap()
    }
}

struct DouyinMonitor;
impl Monitor for DouyinMonitor {
    fn start_once(
        &self,
        url: &str,
        cookies: Option<&str>,
        show_offset: Option<&str>,
        is_new_offset: Option<&str>,
    ) -> (Vec<NewestVideo>, String) {
        // https://www.douyin.com/user/MS4wLjABAAAA
        let _url = Url::parse(url).unwrap();
        // /user/MS4wLjABAAAA
        let path = _url.path();
        // MS4wLjABAAAA
        let id = &path[6..path.len()];
        let query = format!("aid=6383&sec_user_id={}&count=10&max_cursor=0&cookie_enabled=true&platform=PC&downlink=10", id);

        // x-bogus
        let x_bogus = self.calc_x_bogus(&query, DEFAULT_USER_AGENT);
        let query = format!("{}&X-Bogus={}", query, x_bogus);

        let api = format!("{}?{}", DOUYIN_MONITOR_API, query);

        let response = reqwest::blocking::Client::new()
            .get(api)
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .header(COOKIE, cookies.unwrap_or(""))
            .send()
            .unwrap();

        let json: serde_json::Value = response
            .json()
            .and_then(|j| Ok(j))
            .unwrap_or(serde_json::Value::Null);

        let mut videos = Vec::new();
        let mut next_offset: u64 = 0;

        if let Some(vlist) = json["aweme_list"].as_array() {
            for video in vlist {
                let id = video["aweme_id"].as_str().unwrap();
                let title = video["desc"].as_str().unwrap();
                let url = format!("https://www.douyin.com/video/{}", id);
                let date = video["create_time"].as_u64().unwrap() * 1000;

                // offset
                if let Some(offset) = show_offset {
                    if date <= offset.parse::<u64>().unwrap() {
                        continue;
                    }
                }

                let is_new = if let Some(offset) = is_new_offset {
                    date > offset.parse::<u64>().unwrap()
                } else {
                    true
                };

                videos.push(NewestVideo {
                    id: id.to_string(),
                    title: title.to_string(),
                    url,
                    date: date.to_string(),
                    is_new,
                });

                if next_offset == 0 {
                    next_offset = date;
                } else if date > next_offset {
                    next_offset = date;
                }
            }
        } else {
            println!("{:?}", json);
        }

        (videos, next_offset.to_string())
    }
}

impl DouyinMonitor {
    fn calc_x_bogus(&self, query: &str, user_agent: &str) -> String {
        let context = Context::new().unwrap();
        context
            .eval(
                format!(
                    "{}\n;sign(`{}`, `{}`);",
                    x_bogus_js::X_BOGUS_JS,
                    query,
                    user_agent
                )
                .as_str(),
            )
            .unwrap()
            .into_string()
            .unwrap()
    }
}
