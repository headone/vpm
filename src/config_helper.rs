use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use toml::de::Error;
use toml::value::{Date, Datetime};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub assets: Option<Vec<Asset>>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Asset {
    pub name: Option<String>,
    pub link: String,
    // 最大长度为 3
    // 分别记录 今日、上次、上上次 的偏移量
    pub offsets: Option<Vec<Offset>>,
}
impl Asset {
    pub fn get_id(&self) -> String {
        // name + link base64
        let no_name = "NoN".to_string();
        let name = self.name.as_ref().unwrap_or(&no_name);
        let link = &self.link;
        let id = format!("{}{}", name, link);
        return base64::encode(id);
    }
}
pub trait AssetVec {
    fn get_by_id(&mut self, id: &str) -> Option<&mut Asset>;
}
impl AssetVec for Vec<Asset> {
    fn get_by_id(&mut self, id: &str) -> Option<&mut Asset> {
        for asset in self {
            if asset.get_id().as_str() == id {
                return Some(asset);
            }
        }
        return None;
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Offset {
    pub date: Datetime,
    pub mark: String,
}
pub trait OffsetVec {
    fn get_newest_offset(&self, not_today: bool) -> Option<&Offset>;
    fn get_oldest_offset(&self, not_today: bool) -> Option<&Offset>;
    fn remove_newest_offset(&mut self);
    fn remove_oldest_offset(&mut self);
}
impl OffsetVec for Vec<Offset> {
    fn get_newest_offset(&self, not_today: bool) -> Option<&Offset> {
        if self.is_empty() {
            return None;
        }

        // 对比日期，返回最新的日期
        let mut newest: Option<&Offset> = None;
        for offset in self {
            if not_today {
                let today = chrono::Local::now().date_naive();
                let today = naive_date_to_date(today);
                if offset.date.date.unwrap() == today {
                    continue;
                }
            }
            if newest.is_none() {
                newest = Some(offset);
            } else if offset.date > newest.unwrap().date {
                newest = Some(offset);
            }
        }
        return newest;
    }

    fn get_oldest_offset(&self, not_today: bool) -> Option<&Offset> {
        if self.is_empty() {
            return None;
        }

        // 对比日期，返回最旧的日期
        let mut oldest: Option<&Offset> = None;
        for offset in self {
            if not_today {
                let today = chrono::Local::now().date_naive();
                let today = naive_date_to_date(today);
                if offset.date.date.unwrap() == today {
                    continue;
                }
            }
            if oldest.is_none() {
                oldest = Some(offset);
            } else if offset.date < oldest.unwrap().date {
                oldest = Some(offset);
            }
        }
        return oldest;
    }

    fn remove_newest_offset(&mut self) {
        // 对比日期，删除最新的日期
        let date = {
            let newest = self.get_newest_offset(false);
            match newest {
                None => return,
                Some(offset) => offset.date.date,
            }
        };
        self.retain(|offset| offset.date.date != date);
    }

    fn remove_oldest_offset(&mut self) {
        // 对比日期，删除最旧的日期
        let date = {
            let oldest = self.get_oldest_offset(false);
            match oldest {
                None => return,
                Some(offset) => offset.date.date,
            }
        };
        self.retain(|offset| offset.date.date != date);
    }
}

// default config file path
const DEFAULT_CONFIG_PATH: &str = "config.toml";

pub fn read_config(path: Option<&str>) -> Result<Config, Error> {
    let config_path = path.unwrap_or(DEFAULT_CONFIG_PATH);
    touch_config(config_path);
    let config = std::fs::read_to_string(config_path).unwrap();
    let config: Config = toml::from_str(&config)?;
    return Ok(config);
}

pub fn save_config(config: &Config, path: Option<&str>) -> Result<(), std::io::Error> {
    let config_path = path.unwrap_or(DEFAULT_CONFIG_PATH);
    touch_config(config_path);
    let config = toml::to_string(config).unwrap();
    std::fs::write(config_path, config)
}

pub fn touch_config(path: &str) {
    // 检查文件是否存在，不存在则创建
    if !std::path::Path::new(path).exists() {
        std::fs::write(path, "").expect(&format!("Failed to create file: {}", path));
    }
}

pub fn update_offset(asset: &mut Asset, offset_mark: &str) {
    if asset.offsets.is_none() {
        asset.offsets = Some(vec![]);
    }
    let offset = asset.offsets.as_mut().unwrap();

    let today = chrono::Local::now().date_naive();
    let today = Datetime::from(naive_date_to_date(today));

    if let Some(newest_offset) = offset.get_newest_offset(false) {
        if newest_offset.mark == offset_mark {
            return;
        }

        if newest_offset.date == today {
            offset.remove_newest_offset();
        }
        offset.push(Offset {
            date: today,
            mark: offset_mark.to_string(),
        });
    } else {
        offset.push(Offset {
            date: today,
            mark: offset_mark.to_string(),
        });
    }

    if offset.len() > 3 {
        offset.remove_oldest_offset();
    }
}

fn naive_date_to_date(date: NaiveDate) -> Date {
    Date {
        year: date.year() as u16,
        month: date.month() as u8,
        day: date.day() as u8,
    }
}

fn date_to_naive_date(date: Date) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32).unwrap()
}
