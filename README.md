# beepkg

基于 MinIO 或其他兼容 S3 API 存储服务的通用包管理工具。用于存储、分享和管理各种语言的软件包。

## 特性

- 支持多种语言和包格式
- 列出可用的包
- 推送包到存储服务
- 从存储服务拉取包
- 测试存储服务和 bucket 的连接和可用性
- 支持从 .env 文件加载配置

## 安装

克隆仓库并编译:

```bash
git clone <仓库URL>
cd beepkg
cargo build --release
```

## 配置

工具支持两种配置方式：命令行参数和环境变量。环境变量可以通过 `.env` 文件设置，支持以下配置项：

```
MINIO_ENDPOINT=play.min.io
MINIO_BUCKET=packages
MINIO_ACCESS_KEY=your_access_key
MINIO_SECRET_KEY=your_secret_key
```

### 端点 URL 格式

端点 URL 可以使用以下格式：

- 带协议前缀：`http://192.168.7.100:9004` 或 `https://play.min.io`
- 不带协议前缀：`play.min.io` (默认使用 HTTPS)

如果 MinIO 服务器使用非默认端口，请确保在 URL 中包含端口号，例如 `http://192.168.7.100:9004`。

## 使用说明

### 列出可用包

```bash
cargo run --bin beepkg -- test --endpoint http://192.168.7.100:9005 --bucket devregistry
```

例如:
```bash
cargo run --bin beepkg -- list --endpoint http://192.168.7.100:9005 --bucket devregistry
cargo run --bin beepkg -- list --endpoint play.min.io --bucket packages
```

### 推送包

```bash
cargo run --bin beepkg -- push --package <包目录路径> [--key <访问密钥>] [--secret <密钥>]
```

如果不提供访问密钥和密钥，工具将使用匿名访问或环境变量中的凭证。

例如:
```bash
cargo run --bin beepkg -- push --package ./my-package --key minio --secret minio123
cargo run --bin beepkg -- push --package ./my-package
```

### 拉取包

```bash
cargo run --bin beepkg -- pull <包名称@版本> [--output <输出目录>]
```

如果不指定输出目录，将拉取到当前目录下的 `package` 文件夹。

例如:
```bash
cargo run --bin beepkg -- pull my-package@1.0.0 --output ./downloaded-packages
```

### 测试连接

```bash
cargo run --bin beepkg -- test [--endpoint <存储端点>] [--bucket <桶名称>] [--key <访问密钥>] [--secret <密钥>]
```

如果不指定参数，工具将使用 .env 文件或环境变量中的配置。

例如:
```bash
cargo run --bin beepkg -- test --endpoint http://192.168.7.100:9005 --bucket devregistry
```

## 包格式

包应该按照以下结构组织:

```
package-dir/
  ├── pack.toml    # 包元数据 (推荐格式)
  ├── pack.json    # 包元数据 (兼容格式)
  ├── src/         # 源代码
  └── ...          # 其他文件和目录
```

`pack.toml` 文件包含包的元数据，格式如下:

```toml
name = "package-name"
version = "1.0.0"
author = "作者名"
description = "包描述"
includes = ["src/*", "config/"]
excludes = ["temp/", "*.log"]

[dependencies]
other-package = "^1.0.0"
```

为了兼容性，也支持 `pack.json` 格式:

```json
{
  "name": "package-name",
  "version": "1.0.0",
  "author": "作者名",
  "description": "包描述",
  "includes": ["src/*", "config/"],
  "excludes": ["temp/", "*.log"],
  "dependencies": {
    "other-package": "^1.0.0"
  }
}
```

注意: 当两种格式同时存在时，系统会优先读取 `pack.toml` 文件。

## 示例

### 创建测试包

使用TOML格式（推荐）:
```bash
mkdir -p test-package/src
echo 'name = "test-package"
version = "0.1.0"
author = "测试用户"
description = "这是一个测试包"
includes = ["src/*"]
excludes = ["*.log"]

[dependencies]
' > test-package/pack.toml
echo 'print("Hello, World!")' > test-package/src/main.py
```

或者使用JSON格式（兼容）:
```bash
mkdir -p test-package/src
echo '{
  "name": "test-package",
  "version": "0.1.0",
  "author": "测试用户",
  "description": "这是一个测试包",
  "includes": ["src/*"],
  "excludes": ["*.log"],
  "dependencies": {}
}' > test-package/pack.json
echo 'print("Hello, World!")' > test-package/src/main.py
```

### 上传测试包

```bash
cargo run --bin beepkg -- push --package ./test-package
```

### 下载测试包

```bash
cargo run --bin beepkg -- pull test-package@0.1.0 --output ./downloaded
```

### 测试连接

```bash
cargo run --bin beepkg -- test
```

## 环境变量

- `MINIO_ENDPOINT`: MinIO 服务的端点 URL (例如 `play.min.io`)
- `MINIO_BUCKET`: 存储包的桶名称 (默认为 `packages`)
- `MINIO_ACCESS_KEY`: 访问密钥 (如果需要认证)
- `MINIO_SECRET_KEY`: 密钥 (如果需要认证)

## 开发笔记

- 工具使用 `rusty-s3` 库与 S3 兼容存储交互
- 包将被打包为 zip 文件，命名为 `<name>-<version>.zip`
- 拉取包时会验证元数据是否与请求的包名和版本匹配
- 支持多种语言包格式，包括但不限于 Python, JavaScript, Rust 等
