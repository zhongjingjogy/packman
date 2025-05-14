# beepkg

A universal package management tool based on MinIO or other S3 API compatible storage services. Used for storing, sharing and managing packages in various languages.

## Risk Warning

⚠️ **Important Notes** ⚠️

1. This project is built with AI assistance and contains a high percentage of AI-generated code, which may have unexpected behaviors or errors. Please use with caution.
2. All current features are under development and not ready for production use. For testing and evaluation purposes only.

## Features

- Support for multiple languages and package formats
- List available packages
- Push packages to storage service
- Pull packages from storage service
- Test connection and availability of storage service and bucket
- Support loading configuration from .env file

## Installation

Clone the repository and build:

```bash
git clone <repository URL>
cd beepkg
cargo build --release
```

## Configuration

The tool supports two configuration methods: command line parameters and environment variables. Environment variables can be set via `.env` file with following items:

```
S3_ENDPOINT=play.min.io
S3_BUCKET=packages
S3_ACCESS_KEY=your_access_key
S3_SECRET_KEY=your_secret_key
```

### Endpoint URL Format

Endpoint URL can use following formats:
- With protocol prefix: `http://192.168.7.100:9004` or `https://play.min.io`
- Without protocol prefix: `play.min.io` (uses HTTPS by default)

If MinIO server uses non-default port, make sure to include port number in URL, e.g. `http://192.168.7.100:9004`.

## Usage

### List available packages

```bash
cargo run --bin beepkg -- test --endpoint http://192.168.7.100:9005 --bucket devregistry
```

Example:
```bash
cargo run --bin beepkg -- list --endpoint http://192.168.7.100:9005 --bucket devregistry
cargo run --bin beepkg -- list --endpoint play.min.io --bucket packages
```

### Push package

```bash
cargo run --bin beepkg -- push --package <package directory path> [--key <access key>] [--secret <secret key>]
```

If access key and secret key are not provided, the tool will use anonymous access or credentials from environment variables.

Example:
```bash
cargo run --bin beepkg -- push --package ./my-package --key minio --secret minio123
cargo run --bin beepkg -- push --package ./my-package
```

### Pull package

```bash
cargo run --bin beepkg -- pull <package name@version> [--output <output directory>]
```

If output directory is not specified, package will be pulled to `package` folder under current directory.

Example:
```bash
cargo run --bin beepkg -- pull my-package@1.0.0 --output ./downloaded-packages
```

### Test connection

```bash
cargo run --bin beepkg -- test [--endpoint <storage endpoint>] [--bucket <bucket name>] [--key <access key>] [--secret <secret key>]
```

If no parameters are specified, the tool will use configuration from .env file or environment variables.

Example:
```bash
cargo run --bin beepkg -- test --endpoint http://192.168.7.100:9005 --bucket devregistry
```

## Package Format

Packages should be organized in following structure:

```
package-dir/
  ├── pack.toml    # Package metadata (recommended format)
  ├── pack.json    # Package metadata (compatible format)
  ├── src/         # Source code
  └── ...          # Other files and directories
```

`pack.toml` file contains package metadata in following format:

```toml
name = "package-name"
version = "1.0.0"
author = "Author name"
description = "Package description"
includes = ["src/*", "config/"]
excludes = ["temp/", "*.log"]

[dependencies]
other-package = "^1.0.0"
```

For compatibility, `pack.json` format is also supported:

```json
{
  "name": "package-name",
  "version": "1.0.0",
  "author": "Author name",
  "description": "Package description",
  "includes": ["src/*", "config/"],
  "excludes": ["temp/", "*.log"],
  "dependencies": {
    "other-package": "^1.0.0"
  }
}
```

Note: When both formats exist, system will prioritize reading `pack.toml` file.

## Examples

### Create test package

Using TOML format (recommended):
```bash
mkdir -p test-package/src
echo 'name = "test-package"
version = "0.1.0"
author = "Test User"
description = "This is a test package"
includes = ["src/*"]
excludes = ["*.log"]

[dependencies]
' > test-package/pack.toml
echo 'print("Hello, World!")' > test-package/src/main.py
```

Or using JSON format (compatible):
```bash
mkdir -p test-package/src
echo '{
  "name": "test-package",
  "version": "0.1.0",
  "author": "Test User",
  "description": "This is a test package",
  "includes": ["src/*"],
  "excludes": ["*.log"],
  "dependencies": {}
}' > test-package/pack.json
echo 'print("Hello, World!")' > test-package/src/main.py
```

### Upload test package

```bash
cargo run --bin beepkg -- push --package ./test-package
```

### Download test package

```bash
cargo run --bin beepkg -- pull test-package@0.1.0 --output ./downloaded
```

### Test connection

```bash
cargo run --bin beepkg -- test
```

## Environment Variables

- `S3_ENDPOINT`: MinIO service endpoint URL (e.g. `play.min.io`)
- `S3_BUCKET`: Bucket name for storing packages (default: `packages`)
- `S3_ACCESS_KEY`: Access key (if authentication required)
- `S3_SECRET_KEY`: Secret key (if authentication required)

## Development Notes

- Tool uses `rusty-s3` library to interact with S3 compatible storage
- Packages will be packed as zip files named `<name>-<version>.zip`
- When pulling packages, metadata will be verified against requested package name and version
- Supports multiple language package formats including but not limited to Python, JavaScript, Rust etc.
- Lint check: `cargo clippy -- -W clippy::pedantic`
