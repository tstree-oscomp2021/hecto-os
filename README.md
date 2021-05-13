由于评测服务器不提供联网环境，使用 `cargo vendor /path/to/vendor` 命令将 crate 包下载到 `vendor` 文件夹中。

```bash
cd boards/qemu-virt-rv64
cargo --version
make qemu
```
