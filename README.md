# Video Platform Monitor

This is a simple command line application that monitors the status of a video platform. It is built using Rust. The application is designed to monitor author video updates on video platforms and provide a simple automatic recording function. The application is designed to be simple and easy to use, and can be easily extended to monitor other platforms.

## Features

- [x] Monitor author video updates
  - [x] Bilibili
    > Quoted from [bilibili-API-collect](https://github.com/SocialSisterYi/bilibili-API-collect)
  - [x] Xigua
  - [x] Kuaishou
  - [x] Douyin
    > Quoted from [Spider_DouYin_Videos](https://github.com/huifeng-kooboo/Spider_DouYin_Videos)
- [x] Automatic recording of video updates
- [x] Support use cookies to access the platform

## Usage

### 1. Install

Download the latest release from the [release page](https://github.com/headone/vpm/releases).

### 2. Configuration

Create a configuration file named `config.toml` in the current directory. The configuration file is used to configure the monitoring platform and the recording function. The following is an example of the configuration file:

```toml
[cookies]
bilibili = "your bilibili cookies"
xigua = "your xigua cookies"

[[assets]]
name = "B站用户"
link = "https://space.bilibili.com/123123123"

[[assets]]
name = "西瓜用户"
link = "https://www.ixigua.com/home/123123123/"

[[assets]]
name = "快手用户"
link = "https://www.kuaishou.com/profile/123xva123asd"
```

### 3. Run

Run the application in the command line:

```shell
vpm
```
## Build

### 1. Clone the repository

```shell
git clone https://github.com/headone/vpm.git
```

### 2. Build

```shell
cargo build --release
```

#### Windows

because of the influence of the upstream project [quickjs-rs](https://github.com/theduke/quickjs-rs?tab=readme-ov-file#windows-support), so if you want to build for windows, use the following command:

```shell
cargo build --release --target x86_64-pc-windows-gnu
```
