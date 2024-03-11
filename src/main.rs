use crate::config_helper::{update_offset, AssetVec, OffsetVec};
use std::io;

mod config_helper;
mod monitor;

fn main() {
    // config path
    let config_path = None;
    // 获取设置
    let config = config_helper::read_config(config_path).unwrap();

    // 判断配置
    let _config = config.clone();
    if _config.assets.is_none() || _config.assets.unwrap().is_empty() {
        eprintln!("No assets found in config file.");
        // 等待结束
        println!("Press Enter to exit...");
        io::stdin().read_line(&mut String::new()).unwrap();
        return;
    }
    let mut _config = config.clone();

    // 遍历资产配置
    for asset in &config.assets.unwrap() {
        // 输出资产名称
        println!(
            "[{}]({})'s new videos",
            asset.name.as_ref().unwrap_or(&"NoN".to_string()),
            asset.link
        );

        // 处理偏移量
        let show_offset = match &asset.offsets {
            None => None,
            Some(_offsets) => _offsets.get_oldest_offset(true).map(|o| o.mark.as_str()),
        };
        let is_new_offset = match &asset.offsets {
            None => None,
            Some(_offsets) => _offsets.get_newest_offset(true).map(|o| o.mark.as_str()),
        };

        // 获取最新视频
        match monitor::get_newest_video(asset.link.as_str(), config.cookies.clone(), show_offset, is_new_offset) {
            Ok((videos, next_offset)) => {
                if videos.is_empty() {
                    println!("No new videos found.");
                } else {
                    for video in videos {
                        // parse timestamp ms to date
                        let date = format_date(&video.date);
                        println!(
                            "{} {} | {} | {}",
                            if video.is_new { "+" } else { "-" },
                            video.url,
                            date,
                            video.title
                        );
                    }

                    // 更新偏移量
                    let a = _config.assets.as_mut().unwrap();
                    let a = a.get_by_id(asset.get_id().as_str()).unwrap();
                    update_offset(a, next_offset.as_str());
                }
            }
            Err(err) => {
                eprintln!("Error: {}", err.to_string());
            }
        }

        // 换行
        println!();
    }

    // 更新设置
    config_helper::save_config(&_config, config_path).unwrap();

    // 等待结束
    println!("Press Enter to exit...");
    io::stdin().read_line(&mut String::new()).unwrap();
}

fn format_date(date: &str) -> String {
    chrono::DateTime::from_timestamp(date.parse::<i64>().unwrap() / 1000, 0)
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%m-%d %H:%M")
        .to_string()
}
