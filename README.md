```bash
cd kernel/boards/qemu-virt-rv64

# 运行示例程序 busybox_tests，LOG 等级默认为 debug
make run EXAMPLE=busybox_tests LOG=none

# 单元测试
make test_unit

# 集成测试，并关闭 log
make test_integration TEST=00_syscall_tests LOG=none
```

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
